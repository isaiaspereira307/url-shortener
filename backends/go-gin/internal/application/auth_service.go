package application

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/google/uuid"
	"gorm.io/gorm"

	"url-shortener-go/internal/domain"
	"url-shortener-go/internal/infrastructure"
)

var emailRegex = regexp.MustCompile(`^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`)

func ValidateEmail(email string) bool {
	return emailRegex.MatchString(email)
}

func ValidatePassword(password string) error {
	if len(password) < 8 {
		return fmt.Errorf("password must be at least 8 characters")
	}
	return nil
}

func GenerateSlug(name string) string {
	slug := strings.ToLower(strings.TrimSpace(name))
	var result strings.Builder
	for _, r := range slug {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') {
			result.WriteRune(r)
		} else {
			if result.Len() > 0 && result.String()[result.Len()-1] != '-' {
				result.WriteRune('-')
			}
		}
	}
	s := result.String()
	return strings.Trim(s, "-")
}

func CreateTenant(db *gorm.DB, name string) (*domain.Tenant, error) {
	slug := GenerateSlug(name)
	baseSlug := slug
	counter := 1

	for {
		var count int64
		db.Model(&domain.Tenant{}).Where("slug = ?", slug).Count(&count)
		if count == 0 {
			break
		}
		slug = fmt.Sprintf("%s-%d", baseSlug, counter)
		counter++
	}

	tenant := &domain.Tenant{Name: name, Slug: slug}
	if err := db.Create(tenant).Error; err != nil {
		return nil, err
	}
	return tenant, nil
}

func CreateUser(db *gorm.DB, tenantID uuid.UUID, email, password string) (*domain.User, error) {
	hash, err := infrastructure.HashPassword(password)
	if err != nil {
		return nil, err
	}

	user := &domain.User{
		TenantID:     tenantID,
		Email:        email,
		PasswordHash: hash,
	}
	if err := db.Create(user).Error; err != nil {
		return nil, err
	}
	return user, nil
}

func GetUserByEmail(db *gorm.DB, email string) (*domain.User, error) {
	var user domain.User
	err := db.Where("email = ?", email).First(&user).Error
	if err == gorm.ErrRecordNotFound {
		return nil, nil
	}
	return &user, err
}

func GetUserByID(db *gorm.DB, userID uuid.UUID) (*domain.User, error) {
	var user domain.User
	err := db.Where("id = ?", userID).First(&user).Error
	if err == gorm.ErrRecordNotFound {
		return nil, nil
	}
	return &user, err
}
