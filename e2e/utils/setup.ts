/* eslint-disable @typescript-eslint/no-unused-vars */
import { execSync, spawnSync } from 'child_process';
import { FullConfig } from 'playwright/test';

import { dockerCompose } from './docker';

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
  createSnapshot();
};

const globalSetup = (_: FullConfig) => {
  dockerUp();
};

export default globalSetup;
