.PHONY: up down build clean test test-cov logs db-shell help dev dev-rust dev-go test-rust test-go benchmark

help:
	@echo "URL Shortener - Makefile"
	@echo ""
	@echo "Commands:"
	@echo "  make up          - Start all services (Podman Compose)"
	@echo "  make down        - Stop all services"
	@echo "  make build       - Build all images"
	@echo "  make clean       - Remove containers, volumes, and images"
	@echo "  make test        - Run Python backend tests"
	@echo "  make test-cov    - Run Python tests with coverage report"
	@echo "  make test-rust   - Run Rust backend tests"
	@echo "  make test-go     - Run Go backend tests"
	@echo "  make logs        - Follow logs from all services"
	@echo "  make db-shell    - Open psql shell to the database"
	@echo "  make redis-cli   - Open redis-cli"
	@echo "  make dev         - Run Python backend locally (hot reload)"
	@echo "  make dev-rust    - Run Rust backend locally"
	@echo "  make dev-go      - Run Go backend locally"
	@echo "  make install     - Install Python dependencies"
	@echo "  make benchmark   - Run k6 benchmark (requires k6 installed)"
	@echo ""
	@echo "Benchmark:"
	@echo "  make benchmark BACKEND=python  - Benchmark Python backend"
	@echo "  make benchmark BACKEND=rust    - Benchmark Rust backend"
	@echo "  make benchmark BACKEND=go      - Benchmark Go backend"

up:
	podman compose up -d

down:
	podman compose down

build:
	podman compose build

clean:
	podman compose down -v
	podman image prune -f

test:
	cd backends/python-fastapi && python -m pytest -v

test-cov:
	cd backends/python-fastapi && python -m pytest -v --cov=app --cov-report=term-missing

test-rust:
	cd backends/rust-axum && cargo test

test-go:
	cd backends/go-gin && go test ./... -v

logs:
	podman compose logs -f

db-shell:
	podman exec -it url-shortener-db psql -U url_shortener -d url_shortener

redis-cli:
	podman exec -it url-shortener-redis redis-cli

dev:
	cd backends/python-fastapi && uvicorn app.main:app --reload --host 0.0.0.0 --port 8003

dev-rust:
	cd backends/rust-axum && cargo run

dev-go:
	cd backends/go-gin && go run cmd/server/main.go

install:
	cd backends/python-fastapi && pip install -e ".[dev]"

BACKEND ?= python
benchmark:
	k6 run scripts/benchmark.js --env BASE_URL=http://localhost --env BACKEND=$(BACKEND)
