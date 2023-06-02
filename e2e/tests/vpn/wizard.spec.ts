import { expect, test } from '@playwright/test';
import lodash from 'lodash';
import path from 'path';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { apiCreateUsersBulk, apiGetUsers } from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForPromise } from '../../utils/waitForPromise';

test.describe('Setup VPN (wizard) ', () => {
  test.afterEach(() => {
    dockerRestart();
  });

  test('Wizard Import', async ({ page }) => {
    await waitForBase(page);
    // create users to map devices to;
    const users = lodash.range(20).map((id) => ({
      ...testUserTemplate,
      firstName: `test${id}`,
      username: `test${id}`,
    }));
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUsersBulk(page, users);
    await page.goto(routes.base + routes.admin.wizard);
    await page.getByTestId('setup-network').click();
    const navNext = page.getByTestId('wizard-next');
    const navBack = page.getByTestId('wizard-back');
    await page.getByTestId('setup-option-import').click();
    await navNext.click();
    await page.getByTestId('field-name').type('test network');
    await page.getByTestId('field-endpoint').type('127.0.0.1:5051');
    const fileChooserPromise = page.waitForEvent('filechooser');
    await page.getByTestId('upload-config').click();
    const responseImportConfigPromise = page.waitForResponse('**/network/import');
    const fileChooser = await fileChooserPromise;
    const filePath = path.resolve(__dirname.split('e2e/')[0] + 'e2e/assets/test.config');
    fileChooser.setFiles([filePath.toString()]);
    await navNext.click();
    const response = await responseImportConfigPromise;
    expect(response.status()).toBe(201);
    const isNavDisabled = await navBack.isDisabled();
    expect(isNavDisabled).toBe(true);
    let index = 0;
    for (const user of users) {
      const select = page.getByTestId(`map-device-${index}`).locator('.select');
      const floating = page.locator('.select-floating-ui');
      await select.click();
      await waitForPromise(200);
      await floating.waitFor({ state: 'visible' });
      await page
        .locator('.select-floating-ui button > span')
        .locator(`text='${user.firstName + ' ' + user.lastName}'`)
        .click();
      await floating.waitFor({ state: 'hidden' });
      index++;
    }
    const responseMapConfigPromise = page.waitForResponse('**/network/devices');
    await navNext.click();
    const responseMapConfig = await responseMapConfigPromise;
    expect(responseMapConfig.status()).toBe(201);
    const apiUsers = await apiGetUsers(page);
    for (const user of apiUsers.filter((u) => u.username !== 'admin')) {
      expect(user.devices.length).toBe(1);
    }
  });

  // test('Wizard Manual', async ({ page }) => {
  //   await loginBasic(page, defaultUserAdmin);
  //   await page.goto(routes.base + routes.admin.wizard);
  //   const navNext = page.getByTestId('wizard-next');
  //   await page.getByTestId('setup-option-manual').click();
  //   await navNext.click();
  // });
});
