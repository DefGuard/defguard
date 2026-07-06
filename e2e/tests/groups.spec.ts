import { expect, test } from '@playwright/test';

import { routes, testUserTemplate } from '../config';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';

test.describe('Test groups', () => {
  test.beforeEach(() => dockerRestart());

  test('Add user to admin group', async ({ page, browser }) => {
    const testUser = { ...testUserTemplate, username: 'test' };
    await waitForBase(page);
    await createUser(browser, testUser, ['admin']);
    await loginBasic(page, testUser);
    // Wizard is completed in the template snapshot.
    await page.waitForURL('**/admin/overview**', { waitUntil: 'networkidle' });
    expect(page.url()).toContain(routes.admin.overview);
  });
});
