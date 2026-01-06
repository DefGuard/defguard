import { expect, test } from '@playwright/test';

import { testsConfig, testUserTemplate } from '../config';
import { NetworkForm, User } from '../types';
import { apiEnrollmentActivateUser, apiEnrollmentStart } from '../utils/api/enrollment';
import { apiGetUserProfile } from '../utils/api/users';
import {
  createDevice,
  createUserEnrollment,
  password,
  selectEnrollment,
  setPassword,
  setToken,
  validateData,
} from '../utils/controllers/enrollment';
import { loginBasic } from '../utils/controllers/login';
import { disableUser, enableUser } from '../utils/controllers/toggleUserState';
import { createRegularLocation } from '../utils/controllers/vpn/createNetwork';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';

const testNetwork: NetworkForm = {
  name: 'test network',
  address: '10.10.10.1/24',
  endpoint: '127.0.0.1',
  allowed_ips: ['127.1.5.1'],
  port: '5055',
};

test.describe('Create user and enroll him', () => {
  let token: string;
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    const response = await createUserEnrollment(browser, user);
    token = response.token;
    await createRegularLocation(browser, testNetwork);
  });

  test('Complete user enrollment via API', async ({ request, page }) => {
    expect(token).toBeDefined();
    await apiEnrollmentStart(request, token);
    await apiEnrollmentActivateUser(request, password, '+48123456789');

    await waitForBase(page);
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, { username: user.username, password });
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });
  test('Try to complete disabled user enrollment via API', async ({
    page,
    request,
    browser,
  }) => {
    expect(token).toBeDefined();
    await disableUser(browser, user);
    await apiEnrollmentStart(request, token);
    await apiEnrollmentActivateUser(request, password, '+48123456789');

    await waitForBase(page);
    const responsePromise = page.waitForResponse('**/auth');
    await loginBasic(page, { username: user.username, password });
    const response = await responsePromise;
    expect(response.ok()).toBeFalsy();
  });
});
