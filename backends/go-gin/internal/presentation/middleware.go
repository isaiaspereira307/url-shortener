package presentation

import (
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"
	"github.com/google/uuid"

	"url-shortener-go/internal/domain"
	"url-shortener-go/internal/infrastructure"
)

const ContextUserKey = "user"

type AuthUser struct {
	UserID   uuid.UUID
	TenantID uuid.UUID
	Email    string
}

func AuthMiddleware(cfg *infrastructure.Config) gin.HandlerFunc {
	return func(c *gin.Context) {
		authHeader := c.GetHeader("Authorization")
		if authHeader == "" {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "Missing authorization header"})
			c.Abort()
			return
		}

		parts := strings.SplitN(authHeader, " ", 2)
		if len(parts) != 2 || strings.ToLower(parts[0]) != "bearer" {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid authorization header"})
			c.Abort()
			return
		}

		claims, err := infrastructure.VerifyToken(parts[1], cfg)
		if err != nil {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid or expired token"})
			c.Abort()
			return
		}

		if claims.Type != "access" {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid token type"})
			c.Abort()
			return
		}

		userID, err := uuid.Parse(claims.Subject)
		if err != nil {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid user ID"})
			c.Abort()
			return
		}

		tenantID, err := uuid.Parse(claims.TenantID)
		if err != nil {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "Invalid tenant ID"})
			c.Abort()
			return
		}

		c.Set(ContextUserKey, &AuthUser{
			UserID:   userID,
			TenantID: tenantID,
			Email:    claims.Email,
		})
		c.Next()
	}
}

func GetAuthUser(c *gin.Context) (*AuthUser, *domain.AppError) {
	val, exists := c.Get(ContextUserKey)
	if !exists {
		return nil, domain.NewUnauthorizedError("User not found in context")
	}
	user, ok := val.(*AuthUser)
	if !ok {
		return nil, domain.NewInternalError()
	}
	return user, nil
}
