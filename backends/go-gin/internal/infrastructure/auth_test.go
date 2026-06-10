package infrastructure

import (
	"testing"

	"github.com/google/uuid"
)

func TestHashAndVerifyPassword(t *testing.T) {
	password := "securepassword123"
	hash, err := HashPassword(password)
	if err != nil {
		t.Fatalf("failed to hash password: %v", err)
	}

	if !VerifyPassword(password, hash) {
		t.Error("expected password verification to succeed")
	}

	if VerifyPassword("wrongpassword", hash) {
		t.Error("expected password verification to fail for wrong password")
	}
}

func TestPasswordHashDifferentEachTime(t *testing.T) {
	hash1, _ := HashPassword("samepassword")
	hash2, _ := HashPassword("samepassword")

	if hash1 == hash2 {
		t.Error("expected different hashes for same password")
	}
}

func TestCreateAndVerifyAccessToken(t *testing.T) {
	cfg := &Config{
		JWTSecret:          "super-secret-key-change-this-in-production-32chars",
		JWTAccessExpireMin: 15,
	}

	userID := uuid.New()
	tenantID := uuid.New()

	token := CreateAccessToken(cfg, userID, tenantID, "user@example.com")
	claims, err := VerifyToken(token, cfg)
	if err != nil {
		t.Fatalf("failed to verify token: %v", err)
	}

	if claims.Subject != userID.String() {
		t.Errorf("expected subject %s, got %s", userID, claims.Subject)
	}
	if claims.TenantID != tenantID.String() {
		t.Errorf("expected tenant_id %s, got %s", tenantID, claims.TenantID)
	}
	if claims.Email != "user@example.com" {
		t.Errorf("expected email user@example.com, got %s", claims.Email)
	}
	if claims.Type != "access" {
		t.Errorf("expected type access, got %s", claims.Type)
	}
}

func TestCreateAndVerifyRefreshToken(t *testing.T) {
	cfg := &Config{
		JWTSecret:            "super-secret-key-change-this-in-production-32chars",
		JWTRefreshExpireDays: 7,
	}

	userID := uuid.New()
	tenantID := uuid.New()

	token := CreateRefreshToken(cfg, userID, tenantID)
	claims, err := VerifyToken(token, cfg)
	if err != nil {
		t.Fatalf("failed to verify token: %v", err)
	}

	if claims.Type != "refresh" {
		t.Errorf("expected type refresh, got %s", claims.Type)
	}
}

func TestVerifyInvalidToken(t *testing.T) {
	cfg := &Config{JWTSecret: "secret"}
	_, err := VerifyToken("invalid.token.here", cfg)
	if err == nil {
		t.Error("expected error for invalid token")
	}
}

func TestGenerateTOTPSecret(t *testing.T) {
	secret := GenerateTOTPSecret()
	if secret == "" {
		t.Error("expected non-empty TOTP secret")
	}
}

func TestGenerateTOTPURI(t *testing.T) {
	secret := GenerateTOTPSecret()
	uri := GenerateTOTPURI(secret, "user@example.com", "URL Shortener")
	if uri == "" {
		t.Error("expected non-empty TOTP URI")
	}
}

func TestVerifyTOTPValidCode(t *testing.T) {
	secret := GenerateTOTPSecret()
	if VerifyTOTPCode(secret, "000000") {
		t.Error("expected invalid code to fail")
	}
}

func TestGenerateBackupCodes(t *testing.T) {
	codes := GenerateBackupCodes(8)
	if len(codes) != 8 {
		t.Errorf("expected 8 backup codes, got %d", len(codes))
	}
	for _, code := range codes {
		if len(code) != 8 {
			t.Errorf("expected backup code length 8, got %d", len(code))
		}
	}
}

func TestHashAndVerifyBackupCode(t *testing.T) {
	code := "abcd1234"
	hashed := HashBackupCode(code)

	if hashed == code {
		t.Error("expected hashed code to differ from original")
	}

	if !VerifyBackupCode(code, []string{hashed}) {
		t.Error("expected backup code verification to succeed")
	}

	if VerifyBackupCode("wrongcode", []string{hashed}) {
		t.Error("expected backup code verification to fail for wrong code")
	}
}
