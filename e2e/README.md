# Defguard end-to-end tests powered by Playwright

## Prerequisites

- Docker with compose plugin
- Node
- pnpm

## How to run

If needed, specifiy image tag by setting `IMAGE_TAG` variable:

```bash
export IMAGE_TAG=release-1.5-alpha
```

If port 5432 is already in use on the host, publish the test database on another port by setting `E2E_DB_PORT`:

```bash
export E2E_DB_PORT=15432
```

Pull Docker images:

```bash
docker compose --file ../docker-compose.e2e.yaml pull
```

Install packages:

```bash
pnpm install
```

Install Playwright with Chromium driver:

```bash
npx playwright install --with-deps chromium
```

or

```bash
pnpm playwright install --with-deps chromium
```

Run tests:

```bash
pnpm test
```

Run tests with the browser on screen, and stopping on failure:

```bash
pnpm test --headed --max-failures 1
```
