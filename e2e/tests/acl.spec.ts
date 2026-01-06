import { expect, test } from '@playwright/test';
import { dockerRestart } from '../utils/docker';
import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { Protocols, User } from '../types';
import { waitForBase } from '../utils/waitForBase';
import { loginBasic } from '../utils/controllers/login';
import { createAlias } from '../utils/acl';

test.describe('Test aliases', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
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
