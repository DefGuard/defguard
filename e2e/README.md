# Defguard E2E tests powered by Playwright

## Prerequisites

- Docker
- Docker compose
- Node
- pnpm

## How to run
Pull docker images:
```bash
docker-compose --file ../docker-compose.e2e.yaml pull
```
Install packages:
```bash
pnpm install
```
Install playwright chromium driver:
```bash
npx playwright install --with-deps chromium
```
Run tests:
```bash
pnpm test
```
