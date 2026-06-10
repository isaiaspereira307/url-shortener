FROM golang:1.23-alpine AS builder

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o /url-shortener-go ./cmd/server/main.go

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /url-shortener-go /usr/local/bin/url-shortener-go

EXPOSE 8002

CMD ["/usr/local/bin/url-shortener-go"]
