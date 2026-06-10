package infrastructure

import (
	"crypto/subtle"
	"encoding/base64"
	"fmt"
	"strconv"
	"strings"

	"crypto/rand"

	"golang.org/x/crypto/argon2"
)

type PasswordParams struct {
	Memory      uint32
	Iterations  uint32
	Parallelism uint8
	SaltLength  uint32
	KeyLength   uint32
}

func DefaultPasswordParams() *PasswordParams {
	return &PasswordParams{
		Memory:      64 * 1024,
		Iterations:  3,
		Parallelism: 2,
		SaltLength:  16,
		KeyLength:   32,
	}
}

func HashPassword(password string) (string, error) {
	params := DefaultPasswordParams()

	salt := make([]byte, params.SaltLength)
	if _, err := rand.Read(salt); err != nil {
		return "", err
	}

	hash := argon2.IDKey([]byte(password), salt, params.Iterations, params.Memory, params.Parallelism, params.KeyLength)

	b64Salt := base64.RawStdEncoding.EncodeToString(salt)
	b64Hash := base64.RawStdEncoding.EncodeToString(hash)

	encoded := fmt.Sprintf("$argon2id$v=19$m=%d,t=%d,p=%d$%s$%s",
		params.Memory, params.Iterations, params.Parallelism, b64Salt, b64Hash)
	return encoded, nil
}

func VerifyPassword(password, hash string) bool {
	params, salt, key, err := decodeHash(hash)
	if err != nil {
		return false
	}

	testHash := argon2.IDKey([]byte(password), salt, params.Iterations, params.Memory, params.Parallelism, params.KeyLength)
	return subtle.ConstantTimeCompare(key, testHash) == 1
}

func decodeHash(encodedHash string) (*PasswordParams, []byte, []byte, error) {
	parts := strings.Split(encodedHash, "$")
	if len(parts) != 6 || parts[1] != "argon2id" {
		return nil, nil, nil, fmt.Errorf("invalid hash format")
	}

	var params PasswordParams

	// Parse version
	if parts[2] != "v=19" {
		return nil, nil, nil, fmt.Errorf("unsupported version")
	}

	// Parse m,t,p
	paramParts := strings.Split(parts[3], ",")
	if len(paramParts) != 3 {
		return nil, nil, nil, fmt.Errorf("invalid params")
	}

	m, _ := strconv.ParseUint(strings.TrimPrefix(paramParts[0], "m="), 10, 32)
	t, _ := strconv.ParseUint(strings.TrimPrefix(paramParts[1], "t="), 10, 32)
	p, _ := strconv.ParseUint(strings.TrimPrefix(paramParts[2], "p="), 10, 32)

	params.Memory = uint32(m)
	params.Iterations = uint32(t)
	params.Parallelism = uint8(p)

	salt, err := base64.RawStdEncoding.DecodeString(parts[4])
	if err != nil {
		return nil, nil, nil, err
	}

	key, err := base64.RawStdEncoding.DecodeString(parts[5])
	if err != nil {
		return nil, nil, nil, err
	}

	params.SaltLength = uint32(len(salt))
	params.KeyLength = uint32(len(key))

	return &params, salt, key, nil
}
