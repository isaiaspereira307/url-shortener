package domain

import (
	"time"

	"github.com/google/uuid"
)

type Tenant struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()"`
	Name      string    `gorm:"size:255;not null"`
	Slug      string    `gorm:"size:100;uniqueIndex;not null"`
	CreatedAt time.Time
}

type User struct {
	ID           uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()"`
	TenantID     uuid.UUID `gorm:"type:uuid;not null;index:idx_users_tenant_email,unique"`
	Email        string    `gorm:"size:255;not null;index:idx_users_tenant_email,unique"`
	PasswordHash string    `gorm:"size:255;not null"`
	TOTPSecret   *string   `gorm:"size:32"`
	TOTPEnabled  bool      `gorm:"default:false"`
	CreatedAt    time.Time
	Tenant       Tenant `gorm:"foreignKey:TenantID"`
}

type Link struct {
	ID          uuid.UUID  `gorm:"type:uuid;primaryKey;default:gen_random_uuid()"`
	TenantID    uuid.UUID  `gorm:"type:uuid;not null;index"`
	UserID      *uuid.UUID `gorm:"type:uuid"`
	ShortCode   string     `gorm:"size:20;uniqueIndex;not null"`
	OriginalURL string     `gorm:"type:text;not null"`
	Clicks      int64      `gorm:"default:0"`
	CreatedAt   time.Time
	ExpiresAt   *time.Time
	Tenant      Tenant `gorm:"foreignKey:TenantID"`
	User        User   `gorm:"foreignKey:UserID"`
}

type ClickEvent struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()"`
	LinkID    uuid.UUID `gorm:"type:uuid;not null;index"`
	IP        *string
	UserAgent *string
	Referer   *string
	Country   *string `gorm:"size:2"`
	City      *string `gorm:"size:255"`
	Latitude  *float64
	Longitude *float64
	ISP       *string `gorm:"size:255"`
	ClickedAt time.Time
	Link      Link `gorm:"foreignKey:LinkID"`
}

type Pixel struct {
	ID        uuid.UUID  `gorm:"type:uuid;primaryKey;default:gen_random_uuid()"`
	TenantID  uuid.UUID  `gorm:"type:uuid;not null;index:idx_pixels_tenant"`
	UserID    *uuid.UUID `gorm:"type:uuid"`
	Code      string     `gorm:"size:20;uniqueIndex;not null"`
	Name      *string    `gorm:"size:255"`
	Clicks    int64      `gorm:"default:0"`
	CreatedAt time.Time
	Tenant    Tenant `gorm:"foreignKey:TenantID"`
	User      User   `gorm:"foreignKey:UserID"`
}

type PixelClickEvent struct {
	ID        uuid.UUID `gorm:"type:uuid;primaryKey;default:gen_random_uuid()"`
	PixelID   uuid.UUID `gorm:"type:uuid;not null;index"`
	IP        *string
	UserAgent *string
	Referer   *string
	Country   *string `gorm:"size:2"`
	City      *string `gorm:"size:255"`
	Latitude  *float64
	Longitude *float64
	ISP       *string `gorm:"size:255"`
	ClickedAt time.Time
	Pixel     Pixel `gorm:"foreignKey:PixelID"`
}