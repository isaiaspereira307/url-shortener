package infrastructure

import (
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
)

type Claims struct {
	Subject  string `json:"sub"`
	TenantID string `json:"tenant_id"`
	Email    string `json:"email"`
	Type     string `json:"type"`
	jwt.RegisteredClaims
}

func CreateAccessToken(cfg *Config, userID, tenantID uuid.UUID, email string) string {
	now := time.Now()
	claims := &Claims{
		Subject:  userID.String(),
		TenantID: tenantID.String(),
		Email:    email,
		Type:     "access",
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(now.Add(time.Duration(cfg.JWTAccessExpireMin) * time.Minute)),
			IssuedAt:  jwt.NewNumericDate(now),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	signed, _ := token.SignedString([]byte(cfg.JWTSecret))
	return signed
}

func CreateRefreshToken(cfg *Config, userID, tenantID uuid.UUID) string {
	now := time.Now()
	claims := &Claims{
		Subject:  userID.String(),
		TenantID: tenantID.String(),
		Type:     "refresh",
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(now.Add(time.Duration(cfg.JWTRefreshExpireDays) * 24 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(now),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	signed, _ := token.SignedString([]byte(cfg.JWTSecret))
	return signed
}

func VerifyToken(tokenString string, cfg *Config) (*Claims, error) {
	token, err := jwt.ParseWithClaims(tokenString, &Claims{}, func(t *jwt.Token) (interface{}, error) {
		return []byte(cfg.JWTSecret), nil
	})
	if err != nil {
		return nil, err
	}

	claims, ok := token.Claims.(*Claims)
	if !ok || !token.Valid {
		return nil, jwt.ErrTokenInvalidClaims
	}

	return claims, nil
}
