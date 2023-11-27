import { expect, test } from '@playwright/test';

import { routes, testUserTemplate } from '../config';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForRoute } from '../utils/waitForRoute';

test.describe('Test groups', () => {
  test.beforeEach(() => dockerRestart());

  test.afterAll(() => dockerDown());

  test('Add user to admin group', async ({ page, browser }) => {
    const testUser = { ...testUserTemplate, username: 'test' };
    await waitForBase(page);
    await createUser(browser, testUser, ['admin']);
    await loginBasic(page, testUser);
    await waitForRoute(page, routes.admin.wizard);
    expect(page.url()).toBe(routes.base + routes.admin.wizard);
  });
});
