# Defguard end-to-end tests powered by Playwright

## Prerequisites

- Docker with compose plugin
- Node
- pnpm

## How to run

Set `IMAGE_TAG` to select the default tag for all E2E images. Optional component-specific tags override it and otherwise fall back to `IMAGE_TAG`. For example, use a locally built Core image with the `dev` Edge and Gateway images:

```bash
export IMAGE_TAG=dev
export CORE_IMAGE_TAG=local
```

Set `EDGE_IMAGE_TAG` or `GATEWAY_IMAGE_TAG` the same way when those components need different tags.

Build the local Core image from the repository root. A bare build in a worktree uses `+unknown`; when Git metadata is readable, vergen uses the current commit SHA:

```bash
docker build -t ghcr.io/defguard/defguard:local .
```

To pass the current commit explicitly, use:

```bash
docker build --build-arg VERGEN_GIT_SHA=$(git rev-parse HEAD) -t ghcr.io/defguard/defguard:local .
```

If port 5432 is already in use on the host, publish the test database on another port by setting `E2E_DB_PORT`:

```bash
export E2E_DB_PORT=15432
```

Pull Docker images. If using the locally built Core image, omit `core`:

```bash
docker compose --file ../docker-compose.e2e.yaml pull db edge gateway mailpit
```

Otherwise, pull all images:

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
