package infrastructure

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base32"
	"encoding/hex"
	"fmt"

	"github.com/google/uuid"
	"github.com/pquerna/otp/totp"
)

func GenerateTOTPSecret() string {
	key := make([]byte, 20)
	_, _ = rand.Read(key)
	return base32.StdEncoding.WithPadding(base32.NoPadding).EncodeToString(key)
}

func GenerateTOTPURI(secret, email, issuer string) string {
	return fmt.Sprintf("otpauth://totp/%s:%s?secret=%s&issuer=%s&algorithm=SHA1&digits=6&period=30",
		issuer, email, secret, issuer)
}

func VerifyTOTPCode(secret, code string) bool {
	return totp.Validate(code, secret)
}

func GenerateBackupCodes(count int) []string {
	codes := make([]string, count)
	for i := 0; i < count; i++ {
		b := make([]byte, 4)
		for j := 0; j < 4; j++ {
			b[j] = byte(uuid.New().ID() >> uint(j*8))
		}
		codes[i] = hex.EncodeToString(b)
	}
	return codes
}

func HashBackupCode(code string) string {
	h := sha256.Sum256([]byte(code))
	return hex.EncodeToString(h[:])
}

func VerifyBackupCode(code string, hashedCodes []string) bool {
	hashed := HashBackupCode(code)
	for _, hc := range hashedCodes {
		if hc == hashed {
			return true
		}
	}
	return false
}
