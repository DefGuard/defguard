import { defineConfig, devices, ReporterDescription } from '@playwright/test';

import { routes, testsConfig } from './config';
import { loadEnv } from './utils/loadEnv';

loadEnv();

let reporter:
  | 'html'
  | 'list'
  | 'dot'
  | 'line'
  | 'github'
  | 'json'
  | 'junit'
  | 'null'
  | ReporterDescription[]
  | undefined = [['html', { open: 'never' }]];

if (process.env.SHOW_REPORT) {
  reporter = [['html', { open: 'always' }]];
}

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  globalSetup: './utils/globalSetup',
  timeout: testsConfig.TEST_TIMEOUT * 1000,
  testDir: './tests',
  // Exclude files that consist entirely of skipped tests to avoid Playwright
  // collecting and reporting them as empty suites on every shard.
  testIgnore: [
    '**/enrollment.spec.ts',
    '**/externalopenid.spec.ts',
    '**/externalopenidmfa.spec.ts',
    '**/openid.spec.ts',
    // These wizards use dedicated config files with their own globalSetup.
    '**/autoAdoptionWizard.spec.ts',
    '**/migrationWizard.spec.ts',
  ],
  /* Run tests in files in parallel */
  fullyParallel: false,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: reporter,
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
      permissions: ['clipboard-read', 'clipboard-write'],
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
