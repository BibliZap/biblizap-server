#!/bin/bash
# Apply database migrations for BibliZap tracking system

DB_NAME="biblizap_tracking"
DB_USER="biblizap"
DB_PASSWORD="cmfle25nxm4lgbbfi6m5suabad5vp6ktzdzxwxjoe27z6uwso2vswahoykz6opd7vnlrcqtmhves"
DB_HOST="localhost"
DB_PORT="5432"

echo "Applying migrations to $DB_NAME..."

export PGPASSWORD="$DB_PASSWORD"

psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f migrations/001_tracking_tables.sql

if [ $? -eq 0 ]; then
    echo "✓ Migration applied successfully"
else
    echo "✗ Migration failed"
    exit 1
fi
