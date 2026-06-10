package presentation

import (
	"context"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/go-redis/redis/v8"
	"gorm.io/gorm"
)

type HealthHandler struct {
	db    *gorm.DB
	redis *redis.Client
}

func NewHealthHandler(db *gorm.DB, rdb *redis.Client) *HealthHandler {
	return &HealthHandler{db: db, redis: rdb}
}

// Health godoc
// @Summary Health check
// @Tags health
// @Produce json
// @Success 200 {object} HealthResponse
// @Router /health [get]
func (h *HealthHandler) Health(c *gin.Context) {
	dbStatus := "ok"
	redisStatus := "ok"

	sqlDB, err := h.db.DB()
	if err != nil || sqlDB.Ping() != nil {
		dbStatus = "error"
	}

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	if err := h.redis.Ping(ctx).Err(); err != nil {
		redisStatus = "error"
	}

	status := "healthy"
	if dbStatus != "ok" || redisStatus != "ok" {
		status = "degraded"
	}

	c.JSON(http.StatusOK, HealthResponse{
		Status:    status,
		Service:   "url-shortener-go",
		Database:  dbStatus,
		Redis:     redisStatus,
		Timestamp: time.Now(),
	})
}
