import { defineConfig, devices } from '@playwright/test';

import { routes } from './config';

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: './tests',
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 0 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : 1,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: 'html',
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    baseURL: routes.base,
    trace: 'retain-on-failure',
    viewport: { height: 993, width: 1920 },
    video: {
      mode: 'retain-on-failure',
    },
    screenshot: 'only-on-failure',
    contextOptions: {
      permissions: ['clipboard-read', 'clipboard-write', 'accessibility-events'],
    },
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { height: 993, width: 1920 },
      },
    },
  ],
});
