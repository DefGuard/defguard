import { request } from '@playwright/test';
import http from 'http';

import { testsConfig } from '../config';
import {
  dockerCheckContainersAutoAdoption,
  dockerCreateSnapshotAutoAdoption,
  dockerUpAutoAdoption,
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

// Poll until all components in the auto adoption result report success.
const waitForAutoAdoption = async () => {
  const ctx = await request.newContext({ baseURL: testsConfig.BASE_URL });
  await new Promise<void>((resolve) => {
    const check = async () => {
      try {
        const res = await ctx.get('/api/v1/initial_setup/auto_adoption');
        if (res.ok()) {
          const data = await res.json();
          const results: Record<string, { success: boolean }> =
            data.adoption_result ?? {};
          const keys = Object.keys(results);
          if (keys.length > 0 && keys.every((k) => results[k].success === true)) {
            await ctx.dispose();
            resolve();
            return;
          }
        }
      } catch {
        // Ignore errors and retry.
      }
      setTimeout(check, 2000);
    };
    check();
  });
};

export default async function globalSetupAutoAdoption() {
  loadEnv();

  if (!dockerCheckContainersAutoAdoption()) {
    dockerUpAutoAdoption();
  }

  console.log('Waiting for Core (auto adoption) to be ready...');
  await waitForCore();
  console.log('Waiting for auto adoption components to report success...');
  await waitForAutoAdoption();
  // Snapshot taken BEFORE running the wizard so each test can run the wizard
  // from scratch by calling dockerRestartAutoAdoption().
  console.log('Components ready. Creating pre-wizard snapshot...');
  dockerCreateSnapshotAutoAdoption();
  console.log('Snapshot created.');
}
