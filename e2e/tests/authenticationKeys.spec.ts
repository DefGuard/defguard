import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { User } from '../types';
import { apiCreateUser } from '../utils/api/users';
import { createUser } from '../utils/controllers/createUser';
import { loginBasic } from '../utils/controllers/login';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForRoute } from '../utils/waitForRoute';

test.describe('Authentication keys', () => {
  const user: User = { ...testUserTemplate, username: 'test' };

  test.beforeEach(async ({ browser }) => {
    dockerRestart();
    await createUser(browser, user);
  });

  test.afterAll(() => {
    dockerDown();
  });

  test('Add authentication key', async ({ page }) => {
    const testUser = { ...testUserTemplate, username: 'test' };
    await waitForBase(page);
    await apiCreateUser(page, testUser);
    await loginBasic(page, defaultUserAdmin);

    await page.goto(routes.base + routes.me, {
      waitUntil: 'networkidle',
    });
    await waitForRoute(page, routes.me);

    const keyName = 'Test SSH Key';

    const selectButton = page.getByTestId('add-authentication-key-button');
    selectButton.click();

    await page.getByTestId('field-name').type(keyName);
    const key =
      // eslint-disable-next-line max-len
      'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDHQ45F4kNtPZguK84Jh+UkNm8Ss0zxdRMjddSmXkFDdNUNh/Q7mNnUaf+MR5lzhR77g2CpW4sfbll5SU2w5K5eARFMsb04jCA7N2yW0gkPhaUN++DBnOlvWqXq5IslY4Qnv89HNkpYvv5fDPrVVpLWqIDEwES1A3HL0GWHzl95diip+90O9N5Ar9Bggjx5DS0xDgSOf+3D7ZHDVdLCOF5bGs+EseEsi4ZA2Ygd+ukldIowwMeltGYecnLCLt9p1zW57vqnYB+WoTceu0XouTXu26w1V3O2BY8VJVvBCbS/yuq1fEwBac0LhAFWmos7ypfAzWthiEulragUyNjb5LEjTeK4kskcwuGJVfvWP36FWeN3fyuOR3AijlJnRDyzc1uDolVbGYEBEfPMThmwF3KHDJr4Hq5Vc9vyXWXmb00xpooWr0S7KIuZUJDhp7CYHC0rajVtGhcQfQZdOskKkNLYY8iZcEQVGmVHuhoy4DqYjNWU/eQUs0AZwYz5ooO1MhFliABR6hdJI7kSAY5I/AZiMCMu7nLs/k9LhAvwh4eX3i+fBs2MBYEVjhpGkzGIqAm1mLeI1oxdvnw5NNZCHgHp57HSu1DeXiJvZu62/S4DTcN4B33liu6GVQctje+HCp5F9A9XzO82fDJ/zncy2SlQDqUf3DGMpqvEi8HWEFV/oQ==';
    await page.getByTestId('authentication-key-value').type(key);
    await page.getByTestId('submit-add-authentication-key').click();

    await page.getByTestId('card-authentication-key-value').waitFor({ state: 'visible' });
    expect(await page.getByTestId('authentication-key-name').textContent()).toBe(keyName);

    await page.getByTestId('authentication-key-settings-button').click();
    await page.getByTestId('authentication-key-settings-delete').click();
    await page.getByTestId('confirm-modal-submit').click();

    await page.getByTestId('confirm-modal').waitFor({ state: 'hidden' });
    await page.getByTestId('card-authentication-key-value').waitFor({ state: 'hidden' });
  });
});
