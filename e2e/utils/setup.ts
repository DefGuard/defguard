/* eslint-disable @typescript-eslint/no-unused-vars */
import { execSync, spawnSync } from 'child_process';
import { FullConfig } from 'playwright/test';

import { dockerCheckContainers, dockerCompose } from './docker';

// use spawnSync to avoid quote escape issues on windows

const waitForDb = () => {
  // After waiting, sleep for 3 seconds to let Defguard Core apply migrations.
  const res = spawnSync(
    dockerCompose,
    ['exec', 'db', 'sh', '-c', 'until pg_isready -U defguard; do sleep 1; done; sleep 3'],
    { stdio: 'inherit', shell: true },
  );
  if (res.error) {
    throw res.error;
  }
};

const createSnapshot = () => {
  const res = spawnSync(
    dockerCompose,
    ['exec', 'db', 'pg_dump', '-U', 'defguard', '-Fc', '-f', '/tmp/db.dump', 'defguard'],
    { stdio: 'inherit', shell: true },
  );
  if (res.error) {
    throw res.error;
  }
};

// Start Defguard stack with docker compose.
export const dockerUp = () => {
  const command = `${dockerCompose} up --wait`;
  execSync(command);
  waitForDb();
  createSnapshot();
};

const globalSetup = (_: FullConfig) => {
  if (!dockerCheckContainers()) {
    dockerUp();
  }
};

export default globalSetup;
