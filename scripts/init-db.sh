#!/bin/bash
set -e

echo "Running database migrations..."

until pg_isready -h postgres -U url_shortener -d url_shortener; do
  echo "Waiting for PostgreSQL..."
  sleep 2
done

for migration in /migrations/*.sql; do
  echo "Applying $(basename "$migration")..."
  psql -h postgres -U url_shortener -d url_shortener -f "$migration"
done

echo "All migrations applied successfully."
