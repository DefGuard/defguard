import { execSync } from 'child_process';
import path from 'path';

const defguardPath = __dirname.split('e2e')[0];

const dockerFilePath = path.resolve(defguardPath, 'docker-compose.e2e.yaml');
const dockerCompose = `docker compose -f ${dockerFilePath}`;

// Run a SQL statement in the postgres maintenance database.
const psql = (sql: string) =>
  execSync(`${dockerCompose} exec db psql -U defguard -d postgres -c "${sql}"`);

// Start Defguard stack with docker compose.
export const dockerUp = () => {
  execSync(`${dockerCompose} up --wait`);
  // Wait for DB to be ready and let Core apply migrations before proceeding.
  execSync(`${dockerCompose} exec db sh -c 'until pg_isready; do sleep 1; done; sleep 3'`);
};

// Snapshot the current defguard database as a PostgreSQL template so it can
// be cloned instantly on each test reset. Core is briefly stopped to prevent
// active connections from blocking the template creation.
export const dockerCreateSnapshot = () => {
  execSync(`${dockerCompose} kill core`);
  psql("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'");
  psql('DROP DATABASE IF EXISTS defguard_template');
  psql('CREATE DATABASE defguard_template TEMPLATE defguard OWNER defguard');
  execSync(`${dockerCompose} start core`);
  execSync(`until curl -sf http://localhost:8000/api/v1/health > /dev/null; do sleep 1; done`);
};

export const dockerCheckContainers = (): boolean => {
  const containers = execSync(`${dockerCompose} ps -q`).toString().trim();
  return Boolean(containers.length);
};

export const dockerRestart = () => {
  if (!dockerCheckContainers()) {
    dockerUp();
  } else {
    // SIGKILL core immediately — no grace period needed in tests.
    execSync(`${dockerCompose} kill core`);
    // Terminate any connections PostgreSQL still sees (kernel closes sockets on
    // SIGKILL but PostgreSQL may not have processed the hangup yet).
    psql("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'");
    // Drop and instantly recreate defguard from the template (filesystem-level copy).
    psql('DROP DATABASE defguard');
    psql('CREATE DATABASE defguard TEMPLATE defguard_template OWNER defguard');
    // Start core and wait for it to be healthy.
    execSync(`${dockerCompose} start core`);
    execSync(`until curl -sf http://localhost:8000/api/v1/health > /dev/null; do sleep 1; done`);
  }
};
