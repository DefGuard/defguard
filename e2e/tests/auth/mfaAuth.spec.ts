import { expect, test } from '@playwright/test';
import { TOTP } from 'totp-generator';

import { routes, testUserTemplate } from '../../config';
import { User } from '../../types';
import { createUser } from '../../utils/controllers/createUser';
import { loginBasic } from '../../utils/controllers/login';
import { enableEmailMFA } from '../../utils/controllers/mfa/enableEmail';
import { enableSecurityKey } from '../../utils/controllers/mfa/enableSecurityKey';
import { dockerRestart } from '../../utils/docker';
import { waitForBase } from '../../utils/waitForBase';
import { waitForRoute } from '../../utils/waitForRoute';

const EMAIL_CODE_VALIDITY_TIME = 300;

test.describe('MFA authentication', () => {
  let testUser: User;

  test.beforeEach(() => {
    dockerRestart();
    testUser = { ...testUserTemplate, username: 'test' };
  });

  test('Login with Email TOTP', async ({ page, browser }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { secret } = await enableEmailMFA(browser, testUser);

    await loginBasic(page, testUser);
    await page.goto(routes.base + routes.auth.email);
    const { otp: code } = TOTP.generate(secret, {
      digits: 6,
      period: EMAIL_CODE_VALIDITY_TIME, //FIXME: Probably a bug, email codes should be valid for 60 seconds
    });
    const responsePromise = page.waitForResponse('**/verify');
    await page.getByTestId('field-code').fill(code);
    await page.locator('[type="submit"]').click();
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });

  test('Login with security key', async ({ page, browser, context }) => {
    await waitForBase(page);
    await createUser(browser, testUser);
    const { credentialId, rpId, privateKey, userHandle } = await enableSecurityKey(
      browser,
      testUser,
      'key_name',
    );
    await page.goto(routes.base);
    await waitForRoute(page, routes.auth.login);
    await page.getByTestId('field-username').fill(testUser.username);
    await page.getByTestId('field-password').fill(testUser.password);
    await page.getByTestId('sign-in').click();
    await page.getByTestId('login-with-passkey').waitFor({ state: 'visible' });

    const authenticator = await context.newCDPSession(page);
    await authenticator.send('WebAuthn.enable');
    const { authenticatorId: loginAuthenticatorId } = await authenticator.send(
      'WebAuthn.addVirtualAuthenticator',
      {
        options: {
          protocol: 'ctap2',
          transport: 'usb',
          hasResidentKey: true,
          hasUserVerification: true,
          isUserVerified: true,
        },
      },
    );

    await authenticator.send('WebAuthn.addCredential', {
      authenticatorId: loginAuthenticatorId,
      credential: {
        credentialId,
        isResidentCredential: true,
        rpId,
        privateKey,
        userHandle,
        signCount: 1,
      },
    });
    const responsePromise = page.waitForResponse('**/me');
    await page.getByTestId('login-with-passkey').click();
    const response = await responsePromise;
    expect(response.ok()).toBeTruthy();
  });
});
