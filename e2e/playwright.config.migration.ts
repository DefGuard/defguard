import { defineConfig, devices } from '@playwright/test';

import { testsConfig } from './config';
import { loadEnv } from './utils/loadEnv';

loadEnv();

export default defineConfig({
  globalSetup: './utils/globalSetupMigration',
  timeout: testsConfig.TEST_TIMEOUT * 1000,
  testDir: './tests',
  testMatch: '**/migrationWizard.spec.ts',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [['html', { open: 'never' }]],
  use: {
    baseURL: testsConfig.BASE_URL,
    trace: 'retain-on-failure',
    viewport: { height: 993, width: 1920 },
    video: { mode: 'retain-on-failure' },
    screenshot: 'only-on-failure',
    contextOptions: { permissions: ['clipboard-read', 'clipboard-write'] },
  },
  projects: [
    {
      name: 'migration-wizard',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { height: 993, width: 1920 },
      },
    },
  ],
});
