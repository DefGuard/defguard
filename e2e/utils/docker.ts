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
  execSync(
    `${dockerCompose} exec db sh -c 'until pg_isready; do sleep 1; done; sleep 3'`,
  );
};

// Snapshot the current defguard database as a PostgreSQL template so it can
// be cloned instantly on each test reset. Core is briefly stopped to prevent
// active connections from blocking the template creation.
export const dockerCreateSnapshot = () => {
  execSync(`${dockerCompose} kill core`);
  psql(
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'",
  );
  psql('DROP DATABASE IF EXISTS defguard_template');
  psql('CREATE DATABASE defguard_template TEMPLATE defguard OWNER defguard');
  execSync(`${dockerCompose} start core`);
  execSync(
    `until curl -sf http://localhost:8000/api/v1/health > /dev/null; do sleep 1; done`,
  );
};

export const dockerCheckContainers = (): boolean => {
  const containers = execSync(`${dockerCompose} ps -q`).toString().trim();
  return Boolean(containers.length);
};

export const dockerCheckTemplateExists = (): boolean => {
  try {
    const out = execSync(
      `${dockerCompose} exec db psql -U defguard -d postgres -tAc ` +
        `"SELECT 1 FROM pg_database WHERE datname = 'defguard_template'"`,
    )
      .toString()
      .trim();
    return out === '1';
  } catch {
    return false;
  }
};

export const dockerRestart = () => {
  if (!dockerCheckContainers()) {
    dockerUp();
  } else {
    execSync(`${dockerCompose} kill core`);
    psql(
      "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'",
    );
    psql('DROP DATABASE defguard');
    psql('CREATE DATABASE defguard TEMPLATE defguard_template OWNER defguard');
    execSync(`${dockerCompose} start core`);
    execSync(
      `until curl -sf http://localhost:8000/api/v1/health > /dev/null; do sleep 1; done`,
    );
  }
};

const psqlAutoAdoption = (sql: string) =>
  execSync(
    `${dockerComposeAutoAdoption} exec db psql -U defguard -d postgres -c "${sql}"`,
  );

export const dockerUpAutoAdoption = () => {
  execSync(`${dockerComposeAutoAdoption} up --wait`);
  execSync(
    `${dockerComposeAutoAdoption} exec db sh -c 'until pg_isready; do sleep 1; done; sleep 3'`,
  );
};

export const dockerCheckContainersAutoAdoption = (): boolean => {
  const containers = execSync(`${dockerComposeAutoAdoption} ps -q`).toString().trim();
  return Boolean(containers.length);
};

export const dockerCreateSnapshotAutoAdoption = () => {
  execSync(`${dockerComposeAutoAdoption} kill core`);
  psqlAutoAdoption(
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'",
  );
  psqlAutoAdoption('DROP DATABASE IF EXISTS defguard_template');
  psqlAutoAdoption('CREATE DATABASE defguard_template TEMPLATE defguard OWNER defguard');
  execSync(`${dockerComposeAutoAdoption} start core`);
  execSync(
    `until curl -sf http://localhost:8000/api/v1/health > /dev/null; do sleep 1; done`,
  );
};

export const dockerRestartAutoAdoption = () => {
  if (!dockerCheckContainersAutoAdoption()) {
    dockerUpAutoAdoption();
  } else {
    execSync(`${dockerComposeAutoAdoption} kill core`);
    psqlAutoAdoption(
      "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'",
    );
    psqlAutoAdoption('DROP DATABASE defguard');
    psqlAutoAdoption(
      'CREATE DATABASE defguard TEMPLATE defguard_template OWNER defguard',
    );
    execSync(`${dockerComposeAutoAdoption} start core`);
    execSync(
      `until curl -sf http://localhost:8000/api/v1/health > /dev/null; do sleep 1; done`,
    );
  }
};

// UPDATE rather than DELETE: Wizard::init() calls fetch_one on the singleton row,
// which panics on an empty table. Resetting to active_wizard='none' + completed=false
// lets the wizard detect the existing admin user and activate migration mode normally.
export const dockerCreateMigrationState = () => {
  execSync(`${dockerCompose} kill core`);
  psql(
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'defguard'",
  );
  execSync(
    `${dockerCompose} exec db psql -U defguard -d defguard -c ` +
      `"UPDATE wizard SET active_wizard = 'none', completed = false WHERE is_singleton"`,
  );
};


