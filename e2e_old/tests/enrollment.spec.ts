import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import enrollmentController, {
  createUserEnrollment,
} from '../utils/controllers/enrollment';
import { disableUser } from '../utils/controllers/toggleUserState';
import { createNetwork } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

const testNetwork: NetworkForm = {
  name: 'test network',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  port: '5055',
};

test.describe('Enrollment tests', () => {
  let token: string;
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createNetwork(browser, testNetwork);
    const response = await createUserEnrollment(browser, user);
    token = response.token;
  });

  test('Try to complete enrollment with disabled user', async ({ page, browser }) => {
    expect(token).toBeDefined();
    await waitForBase(page);
    await disableUser(browser, user);
    await page.goto(testsConfig.ENROLLMENT_URL);
    await waitForPromise(2000);
    // Test if we can send the token
    await enrollmentController.startEnrollment(page);
    await enrollmentController.fillTokenForm(token, page);
    const startResponse = page.waitForResponse((response) =>
      response.url().endsWith('/start'),
    );
    await enrollmentController.navNext(page);
    expect((await startResponse).status()).toBe(403);
  });

  test('Complete enrollment flow', async ({ page }) => {
    expect(token).toBeDefined();
    await waitForBase(page);
    await page.goto(testsConfig.ENROLLMENT_URL, {
      waitUntil: 'networkidle',
    });
    await enrollmentController.startEnrollment(page);
    await enrollmentController.fillTokenForm(token, page);
    const startResponse = page.waitForResponse((response) =>
      response.url().endsWith('/start'),
    );
    await enrollmentController.navNext(page);
    expect((await startResponse).ok).toBeTruthy();
    //download
    await page.waitForURL('**/download', {
      waitUntil: 'networkidle',
    });
    await enrollmentController.navNext(page);
    await enrollmentController.confirmClientDownloadModal(page);
    await page.waitForURL('**/client-setup', {
      waitUntil: 'networkidle',
    });
    expect(page.locator('#configure-client-page')).toBeVisible();
  });
});
