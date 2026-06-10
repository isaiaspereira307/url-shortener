package infrastructure

import (
	"os"
	"strconv"

	"github.com/joho/godotenv"
)

type Config struct {
	DatabaseURL          string
	RedisURL             string
	JWTSecret            string
	JWTAlgorithm         string
	JWTAccessExpireMin   int
	JWTRefreshExpireDays int
	ShortCodeLength      int
	ServerHost           string
	ServerPort           string
	AppEnv               string
}

func LoadConfig() *Config {
	_ = godotenv.Load()

	jwtAccessExp, _ := strconv.Atoi(getEnv("JWT_ACCESS_EXPIRE_MINUTES", "15"))
	jwtRefreshExp, _ := strconv.Atoi(getEnv("JWT_REFRESH_EXPIRE_DAYS", "7"))
	shortCodeLen, _ := strconv.Atoi(getEnv("SHORT_CODE_LENGTH", "7"))

	return &Config{
		DatabaseURL:          getEnv("DATABASE_URL", "postgresql://url_shortener:url_shortener_secret@localhost:5432/url_shortener"),
		RedisURL:             getEnv("REDIS_URL", "redis://localhost:6379/0"),
		JWTSecret:            getEnv("JWT_SECRET", "super-secret-key-change-this-in-production-32chars"),
		JWTAlgorithm:         getEnv("JWT_ALGORITHM", "HS256"),
		JWTAccessExpireMin:   jwtAccessExp,
		JWTRefreshExpireDays: jwtRefreshExp,
		ShortCodeLength:      shortCodeLen,
		ServerHost:           getEnv("SERVER_HOST", "0.0.0.0"),
		ServerPort:           getEnv("SERVER_PORT", "8002"),
		AppEnv:               getEnv("APP_ENV", "development"),
	}
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
