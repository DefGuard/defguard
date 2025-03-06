#!/usr/bin/env bash
# Removes all test databases
# Test databases are named with UUID strings

set -eo pipefail

if [ -f .env ]; then
    export $(sed -e 's/#.*//g' .env | xargs)
fi

if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error: psql is not installed."
    exit 1
fi

if [ -z "${DATABASE_URL}" ]; then
    echo "DATABASE_URL is not set"
    exit 1
fi

PATTERN='[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}'

echo "Dropping test databases"

psql "${DATABASE_URL}" -c "copy (select datname from pg_database where datname ~ '$PATTERN') to stdout" | while read dbname; do
    echo "Dropping $dbname"
    psql "${DATABASE_URL}" -c "DROP DATABASE \"$dbname\""
done
echo
echo "Test databases were deleted!"
echo
exit
