import { execSync } from 'child_process';
import path from 'path';

import { testsConfig } from '../config';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');

export const dockerCompose = `docker compose -f ${dockerFilePath}`;

// Run a SQL statement in the postgres maintenance database.
const psql = (sql: string) =>
  execSync(`${dockerCompose} exec db psql -U defguard -d postgres -c "${sql}"`);

// Check whether docker compose containers are currently running.
const dockerCheckContainers = (): boolean => {
  const containers = execSync(`${dockerCompose} ps -q`).toString().trim();
  return Boolean(containers.length);
};

// Poll the core's health endpoint until it returns 200.
export const waitForCore = () => {
  execSync(
    `until curl -sf ${testsConfig.BASE_URL}/api/v1/health > /dev/null; do sleep 2; done`,
    { timeout: 120_000 },
  );
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// Start the stack. Used by globalSetup.
export const dockerUp = () => {
  execSync(`${dockerCompose} up --wait`);
};

// Create a PostgreSQL template database snapshot of the current defguard DB.
// Core is killed first so no active connections block the operation.
// Called once by globalSetup after initial DB state is ready.
export const dockerCreateTemplate = () => {
  execSync(`${dockerCompose} kill core`);
  psql(
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'",
  );
  psql('DROP DATABASE IF EXISTS defguard_template');
  psql('CREATE DATABASE defguard_template TEMPLATE defguard OWNER defguard');
  execSync(`${dockerCompose} start core`);
  waitForCore();
};

// Reset the database to the template snapshot.
// Called before each test (via beforeEach).
export const dockerRestart = () => {
  if (!dockerCheckContainers()) {
    dockerUp();
  }
  execSync(`${dockerCompose} kill core`);
  psql(
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'",
  );
  psql('DROP DATABASE defguard');
  psql('CREATE DATABASE defguard TEMPLATE defguard_template OWNER defguard');
  execSync(`${dockerCompose} start core`);
  waitForCore();
};
