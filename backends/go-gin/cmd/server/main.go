// @title URL Shortener - Go Backend
// @description Go/Gin backend for the URL Shortener portfolio project
// @version 1.0
// @host localhost:8002
// @BasePath /
// @securityDefinitions.apikey Bearer
// @in header
// @name Authorization
// @description Type "Bearer" followed by a space and JWT token. Example: "Bearer eyJhbGciOiJIUzI1NiIs..."
package main

import (
	"log"
	"os"

	"github.com/gin-gonic/gin"
	"gorm.io/driver/postgres"
	"gorm.io/gorm"

	ginSwagger "github.com/swaggo/gin-swagger"
	swaggerFiles "github.com/swaggo/files"
	_ "url-shortener-go/docs"

	"url-shortener-go/internal/domain"
	"url-shortener-go/internal/infrastructure"
	"url-shortener-go/internal/presentation"
)

func main() {
	cfg := infrastructure.LoadConfig()

	db, err := gorm.Open(postgres.Open(cfg.DatabaseURL), &gorm.Config{})
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}

	if err := db.AutoMigrate(&domain.Tenant{}, &domain.User{}, &domain.Link{}, &domain.ClickEvent{}, &domain.Pixel{}, &domain.PixelClickEvent{}); err != nil {
		log.Printf("Warning: AutoMigrate had issues (tables may already exist): %v", err)
	}

	rdb := infrastructure.NewRedis(cfg.RedisURL)

	authHandler := presentation.NewAuthHandler(db, cfg)
	linkHandler := presentation.NewLinkHandler(db, rdb, cfg)
	healthHandler := presentation.NewHealthHandler(db, rdb)

	r := gin.Default()
	r.Use(gin.Recovery())

	auth := r.Group("/api/auth")
	{
		auth.POST("/register", authHandler.Register)
		auth.POST("/login", authHandler.Login)
		auth.POST("/login/2fa", authHandler.Login2FA)
		auth.POST("/refresh", authHandler.Refresh)
	}

	protected := r.Group("/api")
	protected.Use(presentation.AuthMiddleware(cfg))
	{
		protected.POST("/shorten", linkHandler.Shorten)
		protected.GET("/links", linkHandler.ListLinks)
		protected.GET("/links/:short_code/stats", linkHandler.LinkStats)
		protected.DELETE("/links/:short_code", linkHandler.DeleteLink)
		protected.POST("/auth/2fa/setup", authHandler.Setup2FA)
		protected.POST("/auth/2fa/verify", authHandler.Verify2FA)
		protected.POST("/auth/2fa/disable", authHandler.Disable2FA)
		protected.POST("/pixel", linkHandler.CreatePixel)
		protected.GET("/pixels", linkHandler.ListPixels)
		protected.DELETE("/pixel/:code", linkHandler.DeletePixel)
	}

	r.GET("/swagger/*any", ginSwagger.WrapHandler(swaggerFiles.Handler))

	r.GET("/health", healthHandler.Health)
	r.GET("/myip", linkHandler.MyIP)
	r.POST("/check-url", linkHandler.CheckURL)
	r.GET("/pixel/:code.png", linkHandler.ServePixel)
	r.POST("/utm-builder", linkHandler.BuildUTM)
	r.GET("/:short_code", linkHandler.Redirect)

	port := os.Getenv("SERVER_PORT")
	if port == "" {
		port = "8002"
	}

	log.Printf("Starting Go server on port %s", port)
	if err := r.Run(":" + port); err != nil {
		log.Fatalf("Failed to start server: %v", err)
	}
}