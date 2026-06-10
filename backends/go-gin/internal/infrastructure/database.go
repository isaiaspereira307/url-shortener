package infrastructure

import (
	"log"

	"gorm.io/driver/postgres"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"

	"url-shortener-go/internal/domain"
)

func NewDatabase(dsn string) *gorm.DB {
	db, err := gorm.Open(postgres.Open(dsn), &gorm.Config{
		Logger: logger.Default.LogMode(logger.Silent),
	})
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}

	if err := db.AutoMigrate(
		&domain.Tenant{},
		&domain.User{},
		&domain.Link{},
		&domain.ClickEvent{},
	); err != nil {
		log.Fatalf("Failed to auto migrate: %v", err)
	}

	return db
}
