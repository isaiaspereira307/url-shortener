package infrastructure

import (
	"context"
	"log"

	"github.com/go-redis/redis/v8"
)

func NewRedis(url string) *redis.Client {
	opt, err := redis.ParseURL(url)
	if err != nil {
		log.Fatalf("Invalid Redis URL: %v", err)
	}

	rdb := redis.NewClient(opt)
	if err := rdb.Ping(context.Background()).Err(); err != nil {
		log.Fatalf("Failed to connect to Redis: %v", err)
	}

	return rdb
}
