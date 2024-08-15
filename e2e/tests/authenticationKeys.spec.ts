import { expect, test } from '@playwright/test';

import { defaultUserAdmin, routes, testUserTemplate } from '../config';
import { AuthenticationKeyType, User } from '../types';
import { apiCreateUser, apiGetUserAuthKeys } from '../utils/api/users';
import { loginBasic } from '../utils/controllers/login';
import { dockerDown, dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { waitForPromise } from '../utils/waitForPromise';
import { waitForRoute } from '../utils/waitForRoute';

test.describe('Authentication keys', () => {
  const testUser: User = { ...testUserTemplate, username: 'test' };
  const testSshKey = `ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQCG+jXb1VHl8Xvxwz1fFFteFX6+4fdOWgWxEA3++p64/s6iHJ3p6jUBc+4cVJg+p7/YKlYGJfxLT3/nrDPTuJ7l/RzbFrqM424KuA+/ZXCa6pDRmB7K+kjSi1I28HokQEL972yhbrkmfqjfPyPHk8RQX3Uw2f6WsQBWvBPMA9pveN6bh6scC5z9VKIoTKHK76RJxkrN7x59EsF1NyJo6jQDOhiBrVS/z3nUhWm05J7AtJn/r0SCZi0K8bXR2zkArr+hodY2WFwYCvsEp+VFTL+O/16enCK8MMz8xbXYVDuiXo7U/cC7s1dmGWsmkmjTVSs2x0KirmgnVwrbdmi0BtEpK5hLLCRFze33kb1VkgFx2kKbEsMJw9qqC7X9t5xTFqR/WVAOBuwCMyUPxF9rqWx0KGy4Y5aqkNwgviOtAKLbNhHx2ToN2/UMaB8KYY+nlb9Y1aOWTebx3T84MrLwE1Vfbd5qJq99ZWFrcvN6I2xQCyHa1zGgMraeKCwqbu39H7BMKTNeTvfXCR/SoVhFpteT/kiX2Hmufufq4feNnlfcFnLVvFRzEQFxDi7+hMrm87dMci58HQu9QIij80qjZtQaXqgjuyxUCx0hCBM4oFUE3rTfJYX7HzTB83Ugqumu3qv8s0aToXQLXFGN6Hkw+IOBupWZFIfy33JAd5az0+r8rQ== `;
  const testPgpKey = `-----BEGIN PGP PUBLIC KEY BLOCK-----
Version: Keybase OpenPGP v1.0.0

xo0EZdcYugEEAO7b3DjbnGMVLHuAaYNuBnQ9ilfzWqidqLF3P+y7bpyQkFA8Rx3M
VzzfMjpnCgZPNs61HxEWZptriHn8zQjyf7pwwozvXKD6S2IKcMEI4kuhuCaBXVaF
CdU4TjUHkUXPbrh+vX1i61FDMdwgqd0oMdMDClQIH2G8HxoUVhulUrZvABEBAAHN
F3Rlc3QgPHRlc3RAdGVvbml0ZS5jb20+wq0EEwEKABcFAmXXGLoCGy8DCwkHAxUK
CAIeAQIXgAAKCRCmxs4bCo4s1c7yBADiOq+h372eT+RneYPcQnYGtgr3Iuy0EbxM
7h504V6MN2oGNY53jqlMouds1DCUdQFw3Oc02O+jvmvAffz8gy2GannPQw+Te21I
w03hfhHPpuMfdBmuHMueIMShJ7XphL9bTMbkUSTbQI/6navtuaV0iy+Duycxs20W
L/m36MKn/s6NBGXXGLoBBADG/m/XaCgst8Tnv+aQU18lXs/qeFRFgH2LPuvOftJ2
rBKk8Q8mYDZ7GMRE64mkPHPtRUaZNrVgYH4SK35ifgDIV5gD34M3LWJ2n9McDzE6
IrZAGZy7wyUO5q3lqgEGzipjmACvhh2WHW5C58NqYiK7HfgZ5FAsHPWk4JwrTu0d
HwARAQABwsCDBBgBCgAPBQJl1xi6BQkPCZwAAhsuAKgJEKbGzhsKjizVnSAEGQEK
AAYFAmXXGLoACgkQGxeF7CApPf7xPAP/e+FdWLpNkqGRAKHyyGWkpo48gT/671sc
JBu7YCcnndb4TTX9BHhTvgLHifZ8fQDECG8Jx0be1JhA6H1IyunzLWSAr99H8Nkh
WPH6oY2CiIlpApq8pohQidug2ZaN16zBVdjvQTjDtN77eAjJIljMqK0cAG7mgVTG
jBVrx+H0XnWj0QP/XHcPmhCaVl9evm7CgTZVbUQFpOTjsa4H3b29OTBg7xbRPYKt
wlzxjzEXlyTxc23+MVQ7JeqXewO9CuPXVWAHaOArwo+Y34NbM9345iUVq4P6ogiI
B6TFrlXakjO+zlGYKaN4mkrZsX5iV2B85Bkc47KYVvgM1RsI6rMXvdoYjDfOjQRl
1xi6AQQA0TZU9q5cA4niz4w8I8lIbA84hJxa213AJ4Pr9pd/b/ZoroseZaUzNx6N
seAZP8q20jaSEfjQ9vZNYpaqaOxfNSQMEl2uUMnO0sPmT8CdiaTRokRiSE1aNqyb
r7f6F8izNmAncuhJbbYR46M067SOWCiSKlhUELzwd5bY89qKyc8AEQEAAcLAgwQY
AQoADwUCZdcYugUJDwmcAAIbLgCoCRCmxs4bCo4s1Z0gBBkBCgAGBQJl1xi6AAoJ
EMB9pBCqzQVBPDAEAMh7LkYk+5riTW+F6YCW85XAYubQj2goYjL40uJqGVd4l4Yi
XueFSo2XCamqw3qEfp1N2+veGVyn0kHZ7PkH9S7ota+eZ/vBKyT8ciAe1daC6cJq
5aK0/cv8wuUJS/Fuk5jDnC4xHb+XsH3kBVHW3yYgmKldMrHrTrnctZvMvhYYnKUD
/1m6iVr1WPwBeRoMaZ86JFnjUfugOhL+244Q/1HqaQLnCzMCjgARLeq0OMzMJ13W
ajY8ozCCcZ+QDGRFVB7sVl/39qsQDQgWTGCdwqwxEZeFskDhCfvtk3j7lW9NinaM
QW+7CejaY/Essu7DN6HwqwXbipny63b8ct1UXjG02S+Q
=VWAR
-----END PGP PUBLIC KEY BLOCK-----`;

  test.beforeEach(async ({ page }) => {
    dockerRestart();
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await apiCreateUser(page, testUser);
    const url = routes.base + routes.admin.users + '/' + testUser.username;
    await page.goto(url);
    await waitForRoute(page, url);
  });

  test.afterAll(() => {
    dockerDown();
  });

  test('Add authentication key (SSH)', async ({ page }) => {
    await page.getByTestId('add-authentication-key-button').click();
    await page.locator('#add-authentication-key-modal').waitFor({
      state: 'visible',
    });
    const modal = page.locator('#add-authentication-key-modal');
    await modal.getByRole('button', { name: 'SSH', exact: true }).click();
    const form = modal.locator('form');
    await form.getByTestId('field-title').fill('test ssh');
    await form.getByTestId('field-keyValue').fill(testSshKey);
    const responsePromise = page.waitForResponse('**/auth_key');
    await modal.locator('button[type="submit"]').click();
    const response = await responsePromise;
    expect(response.status()).toBe(201);
    const profileKeys = await apiGetUserAuthKeys(page, testUser.username);
    expect(profileKeys.length).toBe(1);
    expect(profileKeys[0].name).toBe('test ssh');
    expect(profileKeys[0].key_type).toBe(AuthenticationKeyType.SSH);
    // check if it can be deleted
    const deletePromise = page.waitForResponse('**/auth_key');
    const card = page.locator('.authentication-key-item');
    card.waitFor({
      state: 'visible',
    });
    await waitForPromise(1000);
    await card.locator('.edit-button').click();
    await page.getByRole('button', { name: 'Delete Key', exact: true }).click();
    await page
      .locator('.modal-content')
      .getByRole('button', { name: 'Delete', exact: true })
      .click();
    const deleteResponse = await deletePromise;
    expect(deleteResponse.status()).toBe(200);
    const afterDeleteKeys = await apiGetUserAuthKeys(page, testUser.username);
    expect(afterDeleteKeys.length).toBe(0);
  });

  test('Add authentication key (GPG)', async ({ page }) => {
    await page.getByTestId('add-authentication-key-button').click();
    await page.locator('#add-authentication-key-modal').waitFor({
      state: 'visible',
    });
    const modal = page.locator('#add-authentication-key-modal');
    await modal.getByRole('button', { name: 'GPG', exact: true }).click();
    const responsePromise = page.waitForResponse('**/auth_key');
    const form = modal.locator('form');
    await form.getByTestId('field-title').fill('test pgp');
    await form.getByTestId('field-keyValue').fill(testPgpKey);
    await modal.locator('button[type="submit"]').click();
    const response = await responsePromise;
    expect(response.status()).toBe(201);
    const profileKeys = await apiGetUserAuthKeys(page, testUser.username);
    expect(profileKeys.length).toBe(1);
    expect(profileKeys[0].name).toBe('test pgp');
    expect(profileKeys[0].key_type).toBe(AuthenticationKeyType.GPG);
    // check if it can be deleted
    const deletePromise = page.waitForResponse('**/auth_key');
    const card = page.locator('.authentication-key-item');
    await card.locator('.edit-button').click();
    await page.getByRole('button', { name: 'Delete Key', exact: true }).click();
    await page
      .locator('.modal-content')
      .getByRole('button', { name: 'Delete', exact: true })
      .click();
    const deleteResponse = await deletePromise;
    expect(deleteResponse.status()).toBe(200);
    const afterDeleteKeys = await apiGetUserAuthKeys(page, testUser.username);
    expect(afterDeleteKeys.length).toBe(0);
  });
});
