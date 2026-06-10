package presentation

import (
	"crypto/md5"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/go-redis/redis/v8"
	"gorm.io/gorm"

	"url-shortener-go/internal/application"
	"url-shortener-go/internal/domain"
	"url-shortener-go/internal/infrastructure"
)

type LinkHandler struct {
	db    *gorm.DB
	redis *redis.Client
	cfg   *infrastructure.Config
}

func NewLinkHandler(db *gorm.DB, rdb *redis.Client, cfg *infrastructure.Config) *LinkHandler {
	return &LinkHandler{db: db, redis: rdb, cfg: cfg}
}

var pixelPNG = []byte{
	0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
	0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
	0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
	0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62, 0x00, 0x00, 0x00, 0x02,
	0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
	0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
}

// Shorten godoc
// @Summary Create short link
// @Tags links
// @Accept json
// @Produce json
// @Param request body ShortenRequest true "URL to shorten"
// @Success 201 {object} LinkResponse
// @Failure 400 {object} map[string]string
// @Failure 429 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Security Bearer
// @Router /api/shorten [post]
func (h *LinkHandler) Shorten(c *gin.Context) {
	var req ShortenRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	parsed, err := url.Parse(req.URL)
	if err != nil || (parsed.Scheme != "http" && parsed.Scheme != "https") {
		c.JSON(http.StatusBadRequest, gin.H{"error": "Invalid URL. Must start with http:// https://"})
		return
	}

	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	urlHash := fmt.Sprintf("%x", md5.Sum([]byte(req.URL)))[:12]

	acquired, err := application.AcquireShortenLock(c.Request.Context(), h.redis, urlHash, 5*time.Second)
	if err != nil || !acquired {
		c.JSON(http.StatusTooManyRequests, gin.H{"error": "Too many concurrent requests. Please try again."})
		return
	}
	defer application.ReleaseShortenLock(c.Request.Context(), h.redis, urlHash)

	link, err := application.CreateLink(h.db, authUser.TenantID, authUser.UserID, req.URL, h.cfg.ShortCodeLength)
	if err != nil {
		if appErr, ok := err.(*domain.AppError); ok {
			c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		} else {
			c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to create link"})
		}
		return
	}

	h.redis.Set(c.Request.Context(), fmt.Sprintf("url:%s", link.ShortCode), link.OriginalURL, 24*time.Hour)

	scheme := "http"
	if c.Request.TLS != nil {
		scheme = "https"
	}
	host := c.Request.Host
	shortURL := fmt.Sprintf("%s://%s/%s", scheme, host, link.ShortCode)

	c.JSON(http.StatusCreated, LinkResponse{
		ID:          link.ID.String(),
		ShortURL:    shortURL,
		OriginalURL: link.OriginalURL,
		ShortCode:   link.ShortCode,
		Clicks:      link.Clicks,
		CreatedAt:   link.CreatedAt,
	})
}

// Redirect godoc
// @Summary Redirect to original URL
// @Tags links
// @Param short_code path string true "Short code"
// @Success 302 {string} string "Redirect"
// @Failure 404 {object} map[string]string
// @Router /{short_code} [get]
func (h *LinkHandler) Redirect(c *gin.Context) {
	shortCode := c.Param("short_code")

	if strings.HasPrefix(shortCode, "px_") || strings.HasSuffix(shortCode, ".png") {
		h.ServePixel(c)
		return
	}

	cached, err := h.redis.Get(c.Request.Context(), fmt.Sprintf("url:%s", shortCode)).Result()
	if err == nil {
		ip := c.ClientIP()
		userAgent := c.GetHeader("User-Agent")
		referer := c.GetHeader("Referer")
		go func() {
			_ = application.RecordClick(h.db, shortCode, ip, userAgent, referer)
		}()
		c.Redirect(http.StatusFound, cached)
		return
	}

	link, err := application.GetLinkByShortCode(h.db, shortCode)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Internal error"})
		return
	}

	if link == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "Link not found"})
		return
	}

	h.redis.Set(c.Request.Context(), fmt.Sprintf("url:%s", shortCode), link.OriginalURL, 24*time.Hour)

	ip := c.ClientIP()
	userAgent := c.GetHeader("User-Agent")
	referer := c.GetHeader("Referer")
	go func() {
		_ = application.RecordClick(h.db, shortCode, ip, userAgent, referer)
	}()

	c.Redirect(http.StatusFound, link.OriginalURL)
}

// ListLinks godoc
// @Summary List user's links
// @Tags links
// @Produce json
// @Param page query int false "Page number" default(1)
// @Param limit query int false "Items per page" default(20)
// @Param sort query string false "Sort field" default(created_at)
// @Param order query string false "Sort order" default(desc)
// @Success 200 {object} LinkListResponse
// @Failure 401 {object} map[string]string
// @Security Bearer
// @Router /api/links [get]
func (h *LinkHandler) ListLinks(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	var params PaginationParams
	if err := c.ShouldBindQuery(&params); err != nil {
		params.Page = 1
		params.Limit = 20
		params.Sort = "created_at"
		params.Order = "desc"
	}

	links, total, err := application.GetLinksByUser(h.db, authUser.TenantID, authUser.UserID, params.Page, params.Limit, params.Sort, params.Order)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to fetch links"})
		return
	}

	scheme := "http"
	if c.Request.TLS != nil {
		scheme = "https"
	}
	host := c.Request.Host

	linkResponses := make([]LinkResponse, len(links))
	for i, link := range links {
		linkResponses[i] = LinkResponse{
			ID:          link.ID.String(),
			ShortURL:    fmt.Sprintf("%s://%s/%s", scheme, host, link.ShortCode),
			OriginalURL: link.OriginalURL,
			ShortCode:   link.ShortCode,
			Clicks:      link.Clicks,
			CreatedAt:   link.CreatedAt,
		}
	}

	c.JSON(http.StatusOK, LinkListResponse{
		Links: linkResponses,
		Total: total,
		Page:  params.Page,
		Limit: params.Limit,
	})
}

// LinkStats godoc
// @Summary Get link statistics
// @Tags links
// @Produce json
// @Param short_code path string true "Short code"
// @Success 200 {object} LinkStatsResponse
// @Failure 404 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Security Bearer
// @Router /api/links/{short_code}/stats [get]
func (h *LinkHandler) LinkStats(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	shortCode := c.Param("short_code")
	stats, err := application.GetLinkStats(h.db, authUser.TenantID, authUser.UserID, shortCode)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to fetch stats"})
		return
	}

	if stats == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "Link not found"})
		return
	}

	c.JSON(http.StatusOK, stats)
}

// DeleteLink godoc
// @Summary Delete a link
// @Tags links
// @Param short_code path string true "Short code"
// @Success 200 {object} map[string]string
// @Failure 404 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Security Bearer
// @Router /api/links/{short_code} [delete]
func (h *LinkHandler) DeleteLink(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	shortCode := c.Param("short_code")
	deleted, err := application.DeleteLink(h.db, authUser.TenantID, authUser.UserID, shortCode)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to delete link"})
		return
	}

	if !deleted {
		c.JSON(http.StatusNotFound, gin.H{"error": "Link not found"})
		return
	}

	h.redis.Del(c.Request.Context(), fmt.Sprintf("url:%s", shortCode))

	c.JSON(http.StatusOK, gin.H{"message": "Link deleted successfully"})
}

// MyIP godoc
// @Summary Get client IP info
// @Tags tools
// @Produce json
// @Success 200 {object} MyIPResponse
// @Router /myip [get]
func (h *LinkHandler) MyIP(c *gin.Context) {
	ip := c.ClientIP()
	if forwarded := c.GetHeader("X-Forwarded-For"); forwarded != "" {
		ip = strings.Split(forwarded, ",")[0]
		ip = strings.TrimSpace(ip)
	} else if realIP := c.GetHeader("X-Real-IP"); realIP != "" {
		ip = realIP
	}

	c.JSON(http.StatusOK, MyIPResponse{
		IP:      ip,
		Country: nil,
		City:    nil,
	})
}

// CheckURL godoc
// @Summary Check URL safety
// @Tags tools
// @Accept json
// @Produce json
// @Param request body URLCheckRequest true "URL to check"
// @Success 200 {object} URLCheckResponse
// @Failure 400 {object} map[string]string
// @Router /check-url [post]
func (h *LinkHandler) CheckURL(c *gin.Context) {
	var req URLCheckRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	parsed, err := url.Parse(req.URL)
	if err != nil || (parsed.Scheme != "http" && parsed.Scheme != "https") {
		c.JSON(http.StatusBadRequest, gin.H{"error": "Invalid URL. Must start with http:// or https://"})
		return
	}

	chain := []RedirectStep{{URL: req.URL, Status: nil}}
	isSafe := true
	var warnings []string

	c.JSON(http.StatusOK, URLCheckResponse{
		OriginalURL:    req.URL,
		FinalURL:       &req.URL,
		RedirectChain:  chain,
		TotalRedirects: 0,
		IsSafe:         isSafe,
		Warnings:       warnings,
		ServerIP:       nil,
		ServerHeaders:  nil,
	})
}

// CreatePixel godoc
// @Summary Create tracking pixel
// @Tags tools
// @Accept json
// @Produce json
// @Param request body PixelCreateRequest true "Pixel data"
// @Success 201 {object} PixelResponse
// @Failure 400 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Security Bearer
// @Router /api/pixel [post]
func (h *LinkHandler) CreatePixel(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	var req PixelCreateRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		req.Name = nil
	}

	pixel, err := application.CreatePixel(h.db, authUser.TenantID, authUser.UserID, req.Name)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to create pixel"})
		return
	}

	scheme := "http"
	if c.Request.TLS != nil {
		scheme = "https"
	}
	host := c.Request.Host
	pixelURL := fmt.Sprintf("%s://%s/%s.png", scheme, host, pixel.Code)

	c.JSON(http.StatusCreated, PixelResponse{
		ID:        pixel.ID.String(),
		Code:      pixel.Code,
		Name:      pixel.Name,
		PixelURL:  pixelURL,
		Clicks:    pixel.Clicks,
		CreatedAt: pixel.CreatedAt,
	})
}

// ListPixels godoc
// @Summary List user's pixels
// @Tags tools
// @Produce json
// @Param page query int false "Page number" default(1)
// @Param limit query int false "Items per page" default(20)
// @Success 200 {object} PixelListResponse
// @Failure 401 {object} map[string]string
// @Security Bearer
// @Router /api/pixels [get]
func (h *LinkHandler) ListPixels(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	var params PaginationParams
	if err := c.ShouldBindQuery(&params); err != nil {
		params.Page = 1
		params.Limit = 20
	}

	pixels, total, err := application.GetPixelsByUser(h.db, authUser.TenantID, authUser.UserID, params.Page, params.Limit)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to fetch pixels"})
		return
	}

	scheme := "http"
	if c.Request.TLS != nil {
		scheme = "https"
	}
	host := c.Request.Host

	pixelResponses := make([]PixelResponse, len(pixels))
	for i, p := range pixels {
		pixelResponses[i] = PixelResponse{
			ID:        p.ID.String(),
			Code:      p.Code,
			Name:      p.Name,
			PixelURL:  fmt.Sprintf("%s://%s/%s.png", scheme, host, p.Code),
			Clicks:    p.Clicks,
			CreatedAt: p.CreatedAt,
		}
	}

	c.JSON(http.StatusOK, PixelListResponse{
		Pixels: pixelResponses,
		Total:  total,
		Page:   params.Page,
		Limit:  params.Limit,
	})
}

// DeletePixel godoc
// @Summary Delete a pixel
// @Tags tools
// @Param code path string true "Pixel code"
// @Success 200 {object} map[string]string
// @Failure 404 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Security Bearer
// @Router /api/pixel/{code} [delete]
func (h *LinkHandler) DeletePixel(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	code := c.Param("code")
	deleted, err := application.DeletePixel(h.db, authUser.TenantID, authUser.UserID, code)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to delete pixel"})
		return
	}

	if !deleted {
		c.JSON(http.StatusNotFound, gin.H{"error": "Pixel not found"})
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "Pixel deleted successfully"})
}

// ServePixel godoc
// @Summary Serve tracking pixel
// @Tags tools
// @Produce png
// @Param code path string true "Pixel code"
// @Success 200 {file} binary
// @Router /pixel/{code}.png [get]
func (h *LinkHandler) ServePixel(c *gin.Context) {
	code := c.Param("short_code")
	if strings.HasSuffix(code, ".png") {
		code = strings.TrimSuffix(code, ".png")
	}

	ip := c.ClientIP()
	if forwarded := c.GetHeader("X-Forwarded-For"); forwarded != "" {
		ip = strings.Split(forwarded, ",")[0]
		ip = strings.TrimSpace(ip)
	}
	userAgent := c.GetHeader("User-Agent")
	referer := c.GetHeader("Referer")

	go func() {
		_ = application.RecordPixelClick(h.db, code, ip, userAgent, referer)
	}()

	c.Header("Content-Type", "image/png")
	c.Header("Cache-Control", "no-store, no-cache, must-revalidate")
	c.Header("Pragma", "no-cache")
	c.Header("Expires", "0")
	c.Data(http.StatusOK, "image/png", pixelPNG)
}

// BuildUTM godoc
// @Summary Build UTM URL
// @Tags tools
// @Accept json
// @Produce json
// @Param request body UTMBuildRequest true "UTM parameters"
// @Success 200 {object} UTMResponse
// @Failure 400 {object} map[string]string
// @Router /utm-builder [post]
func (h *LinkHandler) BuildUTM(c *gin.Context) {
	var req UTMBuildRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	parsed, err := url.Parse(req.URL)
	if err != nil || (parsed.Scheme != "http" && parsed.Scheme != "https") {
		c.JSON(http.StatusBadRequest, gin.H{"error": "Invalid URL. Must start with http:// or https://"})
		return
	}

	params := make(map[string]string)
	q := parsed.Query()

	if req.UTMSource != nil {
		q.Set("utm_source", *req.UTMSource)
		params["utm_source"] = *req.UTMSource
	}
	if req.UTMMedium != nil {
		q.Set("utm_medium", *req.UTMMedium)
		params["utm_medium"] = *req.UTMMedium
	}
	if req.UTMCampaign != nil {
		q.Set("utm_campaign", *req.UTMCampaign)
		params["utm_campaign"] = *req.UTMCampaign
	}
	if req.UTMTerm != nil {
		q.Set("utm_term", *req.UTMTerm)
		params["utm_term"] = *req.UTMTerm
	}
	if req.UTMContent != nil {
		q.Set("utm_content", *req.UTMContent)
		params["utm_content"] = *req.UTMContent
	}

	parsed.RawQuery = q.Encode()

	c.JSON(http.StatusOK, UTMResponse{
		OriginalURL: req.URL,
		UTMURL:     parsed.String(),
		Params:     params,
	})
}