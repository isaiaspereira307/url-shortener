package application

import (
	"context"
	"crypto/rand"
	"fmt"
	"math/big"
	"time"

	"github.com/go-redis/redis/v8"
	"github.com/google/uuid"
	"gorm.io/gorm"

	"url-shortener-go/internal/domain"
)

const alphabet = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
const pixelPrefix = "px_"

func GenerateShortCode(length int) string {
	b := make([]byte, length)
	for i := range b {
		n, _ := rand.Int(rand.Reader, big.NewInt(int64(len(alphabet))))
		b[i] = alphabet[n.Int64()]
	}
	return string(b)
}

func GeneratePixelCode(length int) string {
	return pixelPrefix + GenerateShortCode(length)
}

func AcquireShortenLock(ctx context.Context, rdb *redis.Client, urlHash string, timeout time.Duration) (bool, error) {
	lockKey := fmt.Sprintf("lock:shorten:%s", urlHash)
	return rdb.SetNX(ctx, lockKey, "1", timeout).Result()
}

func ReleaseShortenLock(ctx context.Context, rdb *redis.Client, urlHash string) error {
	lockKey := fmt.Sprintf("lock:shorten:%s", urlHash)
	return rdb.Del(ctx, lockKey).Err()
}

func CreateLink(db *gorm.DB, tenantID, userID uuid.UUID, originalURL string, shortCodeLength int) (*domain.Link, error) {
	for i := 0; i < 3; i++ {
		shortCode := GenerateShortCode(shortCodeLength)
		link := &domain.Link{
			TenantID:    tenantID,
			UserID:      &userID,
			ShortCode:   shortCode,
			OriginalURL: originalURL,
		}
		if err := db.Create(link).Error; err == nil {
			return link, nil
		}
	}
	return nil, domain.NewInternalError()
}

func GetLinkByShortCode(db *gorm.DB, shortCode string) (*domain.Link, error) {
	var link domain.Link
	err := db.Where("short_code = ?", shortCode).First(&link).Error
	if err == gorm.ErrRecordNotFound {
		return nil, nil
	}
	return &link, err
}

func GetLinksByUser(db *gorm.DB, tenantID, userID uuid.UUID, page, limit int, sort, order string) ([]domain.Link, int64, error) {
	if limit > 100 {
		limit = 100
	}
	offset := (page - 1) * limit

	allowedSort := map[string]bool{
		"created_at": true,
		"clicks":     true,
		"short_code": true,
	}
	if !allowedSort[sort] {
		sort = "created_at"
	}
	if order != "asc" && order != "desc" {
		order = "desc"
	}
	orderExpr := sort + " " + order

	var total int64
	db.Model(&domain.Link{}).Where("tenant_id = ? AND user_id = ?", tenantID, userID).Count(&total)

	var links []domain.Link
	err := db.Where("tenant_id = ? AND user_id = ?", tenantID, userID).
		Order(orderExpr).
		Limit(limit).
		Offset(offset).
		Find(&links).Error

	return links, total, err
}

func DeleteLink(db *gorm.DB, tenantID, userID uuid.UUID, shortCode string) (bool, error) {
	result := db.Where("short_code = ? AND tenant_id = ? AND user_id = ?", shortCode, tenantID, userID).
		Delete(&domain.Link{})
	return result.RowsAffected > 0, result.Error
}

func IncrementClicks(db *gorm.DB, shortCode string) error {
	return db.Model(&domain.Link{}).
		Where("short_code = ?", shortCode).
		UpdateColumn("clicks", gorm.Expr("clicks + ?", 1)).Error
}

func RecordClick(db *gorm.DB, shortCode, ip, userAgent, referer string) error {
	var link domain.Link
	if err := db.Where("short_code = ?", shortCode).First(&link).Error; err != nil {
		return err
	}

	db.Model(&domain.Link{}).Where("id = ?", link.ID).
		UpdateColumn("clicks", gorm.Expr("clicks + ?", 1))

	clickEvent := domain.ClickEvent{
		LinkID:    link.ID,
		ClickedAt: time.Now(),
	}

	if ip != "" {
		clickEvent.IP = &ip
	}
	if userAgent != "" {
		clickEvent.UserAgent = &userAgent
	}
	if referer != "" {
		clickEvent.Referer = &referer
	}

	return db.Create(&clickEvent).Error
}

func GetLinkStats(db *gorm.DB, tenantID, userID uuid.UUID, shortCode string) (map[string]interface{}, error) {
	var link domain.Link
	if err := db.Where("short_code = ? AND tenant_id = ? AND user_id = ?", shortCode, tenantID, userID).First(&link).Error; err != nil {
		if err == gorm.ErrRecordNotFound {
			return nil, nil
		}
		return nil, err
	}

	var totalClicks int64
	db.Model(&domain.ClickEvent{}).Where("link_id = ?", link.ID).Count(&totalClicks)

	var uniqueVisitors int64
	db.Model(&domain.ClickEvent{}).Where("link_id = ?", link.ID).
		Select("COUNT(DISTINCT ip)").Scan(&uniqueVisitors)

	type CountryCount struct {
		Country string
		Count   int64
	}
	var countryCounts []CountryCount
	db.Model(&domain.ClickEvent{}).
		Select("country, COUNT(*) as count").
		Where("link_id = ? AND country IS NOT NULL", link.ID).
		Group("country").
		Order("count DESC").
		Limit(20).
		Find(&countryCounts)

	clicksByCountry := make(map[string]int64)
	for _, cc := range countryCounts {
		clicksByCountry[cc.Country] = cc.Count
	}

	var recentClicks []domain.ClickEvent
	db.Where("link_id = ?", link.ID).
		Order("clicked_at DESC").
		Limit(50).
		Find(&recentClicks)

	recentData := make([]map[string]interface{}, 0, len(recentClicks))
	browsers := make(map[string]int64)
	platforms := make(map[string]int64)

	for _, click := range recentClicks {
		entry := map[string]interface{}{
			"ip":         click.IP,
			"country":    click.Country,
			"city":       click.City,
			"user_agent": click.UserAgent,
			"referer":    click.Referer,
			"clicked_at": click.ClickedAt,
		}
		recentData = append(recentData, entry)

		if click.UserAgent != nil {
			ua := *click.UserAgent
			browsers[parseBrowser(ua)]++
			platforms[parsePlatform(ua)]++
		}
	}

	result := map[string]interface{}{
		"short_code":        link.ShortCode,
		"original_url":      link.OriginalURL,
		"total_clicks":      link.Clicks,
		"unique_visitors":   uniqueVisitors,
		"clicks_by_country": clicksByCountry,
		"clicks_by_day":     []interface{}{},
		"recent_clicks":    recentData,
		"browsers":          browsers,
		"platforms":         platforms,
	}

	return result, nil
}

func CreatePixel(db *gorm.DB, tenantID, userID uuid.UUID, name *string) (*domain.Pixel, error) {
	code := GeneratePixelCode(8)
	pixel := &domain.Pixel{
		TenantID: tenantID,
		UserID:   &userID,
		Code:     code,
		Name:     name,
	}
	if err := db.Create(pixel).Error; err != nil {
		return nil, err
	}
	return pixel, nil
}

func GetPixelsByUser(db *gorm.DB, tenantID, userID uuid.UUID, page, limit int) ([]domain.Pixel, int64, error) {
	if limit > 100 {
		limit = 100
	}
	offset := (page - 1) * limit

	var total int64
	db.Model(&domain.Pixel{}).Where("tenant_id = ? AND user_id = ?", tenantID, userID).Count(&total)

	var pixels []domain.Pixel
	err := db.Where("tenant_id = ? AND user_id = ?", tenantID, userID).
		Order("created_at DESC").
		Limit(limit).
		Offset(offset).
		Find(&pixels).Error

	return pixels, total, err
}

func DeletePixel(db *gorm.DB, tenantID, userID uuid.UUID, code string) (bool, error) {
	result := db.Where("code = ? AND tenant_id = ? AND user_id = ?", code, tenantID, userID).
		Delete(&domain.Pixel{})
	return result.RowsAffected > 0, result.Error
}

func RecordPixelClick(db *gorm.DB, code, ip, userAgent, referer string) error {
	var pixel domain.Pixel
	if err := db.Where("code = ?", code).First(&pixel).Error; err != nil {
		return err
	}

	db.Model(&domain.Pixel{}).Where("id = ?", pixel.ID).
		UpdateColumn("clicks", gorm.Expr("clicks + ?", 1))

	clickEvent := domain.PixelClickEvent{
		PixelID:   pixel.ID,
		ClickedAt: time.Now(),
	}

	if ip != "" {
		clickEvent.IP = &ip
	}
	if userAgent != "" {
		clickEvent.UserAgent = &userAgent
	}
	if referer != "" {
		clickEvent.Referer = &referer
	}

	return db.Create(&clickEvent).Error
}

func parseBrowser(userAgent string) string {
	ua := userAgent
	if contains(ua, "Edg") {
		return "Edge"
	}
	if contains(ua, "Chrome") {
		return "Chrome"
	}
	if contains(ua, "Firefox") {
		return "Firefox"
	}
	if contains(ua, "Safari") && !contains(ua, "Chrome") {
		return "Safari"
	}
	if contains(ua, "Opera") || contains(ua, "OPR") {
		return "Opera"
	}
	return "Other"
}

func parsePlatform(userAgent string) string {
	ua := userAgent
	if contains(ua, "Windows") {
		return "Windows"
	}
	if contains(ua, "Mac") {
		return "macOS"
	}
	if contains(ua, "Linux") {
		return "Linux"
	}
	if contains(ua, "Android") {
		return "Android"
	}
	if contains(ua, "iPhone") || contains(ua, "iPad") {
		return "iOS"
	}
	return "Other"
}

func contains(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}