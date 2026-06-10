package presentation

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"gorm.io/gorm"

	"url-shortener-go/internal/application"
	"url-shortener-go/internal/infrastructure"
)

type AuthHandler struct {
	db  *gorm.DB
	cfg *infrastructure.Config
}

func NewAuthHandler(db *gorm.DB, cfg *infrastructure.Config) *AuthHandler {
	return &AuthHandler{db: db, cfg: cfg}
}

// Register godoc
// @Summary Register new user
// @Tags auth
// @Accept json
// @Produce json
// @Param request body RegisterRequest true "Registration data"
// @Success 201 {object} TokenResponse
// @Failure 400 {object} map[string]string
// @Failure 409 {object} map[string]string
// @Failure 500 {object} map[string]string
// @Router /api/auth/register [post]
func (h *AuthHandler) Register(c *gin.Context) {
	var req RegisterRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	if !application.ValidateEmail(req.Email) {
		c.JSON(http.StatusBadRequest, gin.H{"error": "Invalid email format"})
		return
	}

	if err := application.ValidatePassword(req.Password); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	tenant, err := application.CreateTenant(h.db, req.TenantName)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to create tenant"})
		return
	}

	user, err := application.CreateUser(h.db, tenant.ID, req.Email, req.Password)
	if err != nil {
		c.JSON(http.StatusConflict, gin.H{"error": "Email already exists"})
		return
	}

	accessToken := infrastructure.CreateAccessToken(h.cfg, user.ID, tenant.ID, user.Email)
	refreshToken := infrastructure.CreateRefreshToken(h.cfg, user.ID, tenant.ID)

	c.JSON(http.StatusCreated, TokenResponse{
		AccessToken:  accessToken,
		RefreshToken: refreshToken,
		TokenType:    "bearer",
		TOTPRequired: false,
	})
}

// Login godoc
// @Summary Login user
// @Tags auth
// @Accept json
// @Produce json
// @Param request body LoginRequest true "Login credentials"
// @Success 200 {object} TokenResponse
// @Failure 400 {object} map[string]string
// @Failure 401 {object} map[string]string
// @Router /api/auth/login [post]
func (h *AuthHandler) Login(c *gin.Context) {
	var req LoginRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	user, err := application.GetUserByEmail(h.db, req.Email)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Internal error"})
		return
	}

	if user == nil || !infrastructure.VerifyPassword(req.Password, user.PasswordHash) {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid email or password"})
		return
	}

	if user.TOTPEnabled {
		c.JSON(http.StatusOK, TokenResponse{
			AccessToken:  "",
			RefreshToken: "",
			TokenType:    "bearer",
			TOTPRequired: true,
		})
		return
	}

	accessToken := infrastructure.CreateAccessToken(h.cfg, user.ID, user.TenantID, user.Email)
	refreshToken := infrastructure.CreateRefreshToken(h.cfg, user.ID, user.TenantID)

	c.JSON(http.StatusOK, TokenResponse{
		AccessToken:  accessToken,
		RefreshToken: refreshToken,
		TokenType:    "bearer",
		TOTPRequired: false,
	})
}

// Login2FA godoc
// @Summary Login with 2FA
// @Tags auth
// @Accept json
// @Produce json
// @Param request body TotpVerifyRequest true "2FA code"
// @Success 200 {object} TokenResponse
// @Failure 400 {object} map[string]string
// @Failure 401 {object} map[string]string
// @Security Bearer
// @Router /api/auth/login/2fa [post]
func (h *AuthHandler) Login2FA(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	var req TotpVerifyRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	user, err := application.GetUserByID(h.db, authUser.UserID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Internal error"})
		return
	}

	if user == nil || user.TOTPSecret == nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "2FA not enabled"})
		return
	}

	if !application.VerifyTOTP(*user.TOTPSecret, req.Code) {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid 2FA code"})
		return
	}

	accessToken := infrastructure.CreateAccessToken(h.cfg, user.ID, user.TenantID, user.Email)
	refreshToken := infrastructure.CreateRefreshToken(h.cfg, user.ID, user.TenantID)

	c.JSON(http.StatusOK, TokenResponse{
		AccessToken:  accessToken,
		RefreshToken: refreshToken,
		TokenType:    "bearer",
		TOTPRequired: false,
	})
}

// Refresh godoc
// @Summary Refresh access token
// @Tags auth
// @Accept json
// @Produce json
// @Param request body RefreshRequest true "Refresh token"
// @Success 200 {object} TokenResponse
// @Failure 400 {object} map[string]string
// @Failure 401 {object} map[string]string
// @Router /api/auth/refresh [post]
func (h *AuthHandler) Refresh(c *gin.Context) {
	var req RefreshRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	claims, err := infrastructure.VerifyToken(req.RefreshToken, h.cfg)
	if err != nil {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid refresh token"})
		return
	}

	if claims.Type != "refresh" {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid token type"})
		return
	}

	userID, err := uuid.Parse(claims.Subject)
	if err != nil {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid token"})
		return
	}

	user, err := application.GetUserByID(h.db, userID)
	if err != nil || user == nil {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "User not found"})
		return
	}

	accessToken := infrastructure.CreateAccessToken(h.cfg, user.ID, user.TenantID, user.Email)
	newRefreshToken := infrastructure.CreateRefreshToken(h.cfg, user.ID, user.TenantID)

	c.JSON(http.StatusOK, TokenResponse{
		AccessToken:  accessToken,
		RefreshToken: newRefreshToken,
		TokenType:    "bearer",
		TOTPRequired: false,
	})
}

// Setup2FA godoc
// @Summary Setup 2FA for user
// @Tags auth
// @Accept json
// @Produce json
// @Success 200 {object} TotpSetupResponse
// @Failure 400 {object} map[string]string
// @Failure 401 {object} map[string]string
// @Failure 404 {object} map[string]string
// @Security Bearer
// @Router /api/auth/2fa/setup [post]
func (h *AuthHandler) Setup2FA(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	user, err := application.GetUserByID(h.db, authUser.UserID)
	if err != nil || user == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "User not found"})
		return
	}

	if user.TOTPEnabled {
		c.JSON(http.StatusBadRequest, gin.H{"error": "2FA already enabled"})
		return
	}

	result := application.SetupTOTP(user.Email)

	user.TOTPSecret = &result.Secret
	h.db.Save(user)

	c.JSON(http.StatusOK, TotpSetupResponse{
		Secret:      result.Secret,
		QRCodeURI:   result.QRCodeURI,
		BackupCodes: result.BackupCodes,
	})
}

// Verify2FA godoc
// @Summary Verify and enable 2FA
// @Tags auth
// @Accept json
// @Produce json
// @Param request body TotpVerifyRequest true "2FA verification code"
// @Success 200 {object} map[string]string
// @Failure 400 {object} map[string]string
// @Failure 401 {object} map[string]string
// @Failure 404 {object} map[string]string
// @Security Bearer
// @Router /api/auth/2fa/verify [post]
func (h *AuthHandler) Verify2FA(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	var req TotpVerifyRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	user, err := application.GetUserByID(h.db, authUser.UserID)
	if err != nil || user == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "User not found"})
		return
	}

	if user.TOTPSecret == nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "2FA not set up"})
		return
	}

	if !application.VerifyTOTP(*user.TOTPSecret, req.Code) {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid 2FA code"})
		return
	}

	user.TOTPEnabled = true
	h.db.Save(user)

	c.JSON(http.StatusOK, gin.H{"message": "2FA enabled successfully"})
}

// Disable2FA godoc
// @Summary Disable 2FA
// @Tags auth
// @Accept json
// @Produce json
// @Param request body TotpVerifyRequest true "2FA verification code"
// @Success 200 {object} map[string]string
// @Failure 400 {object} map[string]string
// @Failure 401 {object} map[string]string
// @Failure 404 {object} map[string]string
// @Security Bearer
// @Router /api/auth/2fa/disable [post]
func (h *AuthHandler) Disable2FA(c *gin.Context) {
	authUser, appErr := GetAuthUser(c)
	if appErr != nil {
		c.JSON(appErr.Code, gin.H{"error": appErr.Message})
		return
	}

	var req TotpVerifyRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	user, err := application.GetUserByID(h.db, authUser.UserID)
	if err != nil || user == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "User not found"})
		return
	}

	if !user.TOTPEnabled {
		c.JSON(http.StatusBadRequest, gin.H{"error": "2FA not enabled"})
		return
	}

	if user.TOTPSecret == nil || !application.VerifyTOTP(*user.TOTPSecret, req.Code) {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid 2FA code"})
		return
	}

	user.TOTPEnabled = false
	user.TOTPSecret = nil
	h.db.Save(user)

	c.JSON(http.StatusOK, gin.H{"message": "2FA disabled successfully"})
}
