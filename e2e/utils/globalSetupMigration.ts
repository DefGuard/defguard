import { request } from '@playwright/test';
import http from 'http';

import { defaultUserAdmin, testsConfig } from '../config';
import {
  dockerCheckContainers,
  dockerCheckTemplateExists,
  dockerCreateMigrationState,
  dockerCreateSnapshot,
  dockerUp,
} from './docker';
import { loadEnv } from './loadEnv';

const waitForCore = async () => {
  const url = testsConfig.BASE_URL + '/api/v1/health';
  await new Promise<void>((resolve) => {
    const check = () => {
      const req = http.get(url, (res) => {
        if (res.statusCode === 200) {
          resolve();
        } else {
          setTimeout(check, 2000);
        }
      });
      req.on('error', () => setTimeout(check, 2000));
      req.end();
    };
    check();
  });
};

// POST to the initial setup admin endpoint to create the admin user.
const createAdminUser = async () => {
  const ctx = await request.newContext({ baseURL: testsConfig.BASE_URL });
  const res = await ctx.post('/api/v1/initial_setup/admin', {
    data: {
      username: defaultUserAdmin.username,
      password: defaultUserAdmin.password,
      email: defaultUserAdmin.mail,
      first_name: defaultUserAdmin.firstName,
      last_name: defaultUserAdmin.lastName,
      automatically_assign_group: true,
    },
  });
  await ctx.dispose();
  if (!res.ok()) {
    throw new Error(`Failed to create admin user: ${res.status()}`);
  }
};

export default async function globalSetupMigration() {
  loadEnv();

  if (!dockerCheckContainers()) {
    dockerUp();
  }

  if (dockerCheckTemplateExists()) {
    console.log('Migration snapshot already exists, skipping global setup.');
    return;
  }

  console.log('Waiting for Core (migration) to be ready...');
  await waitForCore();
  console.log('Core ready. Creating admin user...');
  await createAdminUser();
  console.log('Preparing migration state...');
  dockerCreateMigrationState();
  dockerCreateSnapshot();
  console.log('Migration snapshot created.');
}
