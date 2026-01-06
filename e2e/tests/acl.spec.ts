import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes } from '../config';
import { Protocols } from '../types';
import { createAlias } from '../utils/acl';
import { loginBasic } from '../utils/controllers/login';
import { dockerRestart } from '../utils/docker';

test.describe('Test aliases', () => {
  // let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    // testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Create alias and check content', async ({ page, browser }) => {
    const name = 'TestAlias';
    const addresses = ['1.2.3.4/24', '10.10.10.10/20', '1.2.4.2'];
    const ports = ['80', '443'];
    const protocols = [Protocols.UDP, Protocols.ICMP];
    await createAlias(browser, name, addresses, ports, protocols);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.firewall.aliases);
    const aliasRow = await page.locator('.virtual-row').filter({ hasText: name });
    await expect(aliasRow).toBeVisible();
    await expect(aliasRow).toContainText(addresses.join(', '));
    await expect(aliasRow).toContainText(ports.join(', '));
    await expect(aliasRow).toContainText(Protocols.UDP);
    await expect(aliasRow).toContainText(Protocols.ICMP);
  });
});
