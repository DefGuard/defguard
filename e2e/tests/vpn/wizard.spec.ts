import { expect, test } from '@playwright/test';
import * as fs from 'fs';
import lodash from 'lodash';
import path from 'path';

import { defaultUserAdmin, routes, testUserTemplate } from '../../config';
import { NetworkForm } from '../../types';
import {
  apiCreateUsersBulk,
  apiGetUserProfile,
  apiGetUsers,
} from '../../utils/api/users';
import { loginBasic } from '../../utils/controllers/login';
import { createNetwork } from '../../utils/controllers/vpn/createNetwork';
import { dockerDown, dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForPromise } from '../../utils/waitForPromise';
import { waitForRoute } from '../../utils/waitForRoute';

test.describe('Setup VPN (wizard) ', () => {
  test.beforeAll(() => {
    dockerRestart();
  });

  test.afterEach(() => {
    dockerRestart();
  });

  test.afterAll(() => {
    dockerDown();
  });

  test('Wizard Import', async ({ page }) => {
    await waitForBase(page);
    // create users to map devices to;
    const users = lodash.range(50).map((id) => ({
      ...testUserTemplate,
      firstName: `test${id}`,
      username: `test${id}`,
      mail: `test${id}@test.com`,
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
    const responseImportConfigPromise = page.waitForResponse('**/import');
    const fileChooser = await fileChooserPromise;
    const filePath = path.resolve(
      __dirname.split('e2e')[0],
      'e2e',
      'assets',
      'test.config'
    );
    fs.accessSync(filePath, fs.constants.F_OK);
    const configData = fs.readFileSync(filePath, null);
    await fileChooser.setFiles([
      {
        name: 'test.config',
        buffer: configData,
        mimeType: 'text/plain',
      },
    ]);
    await navNext.click();
    const response = await responseImportConfigPromise;
    expect(response.status()).toBe(201);
    const isNavDisabled = await navBack.isDisabled();
    expect(isNavDisabled).toBe(true);
    let rowIndex = 0;
    for (const user of users) {
      const selectElement = page.getByTestId(`user-select-${rowIndex}`);
      const selectFloatingExpand = page.locator('.select-floating-ui');
      await selectElement.click();
      await waitForPromise(200);
      await selectFloatingExpand.waitFor({ state: 'visible' });
      await page
        .locator('.select-floating-ui button > span')
        .locator(`text='${user.firstName + ' ' + user.lastName}'`)
        .click();
      await selectFloatingExpand.waitFor({ state: 'hidden' });
      rowIndex++;
    }
    const responseMapConfigPromise = page.waitForResponse('**/devices');
    await navNext.click();
    const responseMapConfig = await responseMapConfigPromise;
    expect(responseMapConfig.status()).toBe(201);
    await waitForRoute(page, routes.admin.overview);
    const apiUsers = await apiGetUsers(page);
    for (const user of apiUsers.filter((u) => u.username !== 'admin')) {
      const userProfile = await apiGetUserProfile(page, user.username);
      expect(userProfile.devices.length).toBe(1);
    }
  });

  test('Wizard Manual', async ({ page, browser }) => {
    await waitForBase(page);
    const network: NetworkForm = {
      name: 'test manual',
      address: '10.10.10.1/24',
      endpoint: '127.0.0.1',
      port: '5055',
    };
    await createNetwork(browser, network);
  });
});
