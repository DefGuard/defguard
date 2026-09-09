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

PATTERN='_sqlx_test_*'

echo "Dropping test databases"

psql "${DATABASE_URL}" -c "copy (SELECT datname FROM pg_database WHERE datname ~ '${PATTERN}') to stdout" | while read dbname; do
    echo "Dropping ${dbname}"
    psql "${DATABASE_URL}" -c "DROP DATABASE \"${dbname}\""
done
echo
echo "Test databases were deleted!"
echo
exit
