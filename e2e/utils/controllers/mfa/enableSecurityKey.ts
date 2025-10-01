import { Browser } from 'playwright';

import { User } from '../../../types';
import { waitForBase } from '../../waitForBase';
import { waitForRoute } from '../../waitForRoute';
import { loginBasic } from '../login';

export type EnableSecurityKeyResult = {
  credentialId: string;
  rpId?: string;
  privateKey: string;
  userHandle?: string;
};

export const enableSecurityKey = async (
  browser: Browser,
  user: User,
  keyName: string,
): Promise<EnableSecurityKeyResult> => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, user);

  const url = routes.base + routes.me;
  await page.goto(url);
  await waitForRoute(page, url);

  await page.getByTestId('edit-user').click();
  await page.getByTestId('edit-security-key').click();
  await page.getByTestId('edit-security-key').click(); // triggering this twice because this button works this way
  await page.waitForTimeout(1000);
  await page.getByTestId('manage-security-keys').click();
  await page.waitForTimeout(1000);

  await page.getByTestId('field-name').fill(keyName);
  await page.getByTestId('add-new-security-key').click();

  const authenticator = await context.newCDPSession(page);
  await authenticator.send('WebAuthn.enable');

  const { authenticatorId } = await authenticator.send(
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
  await page.waitForTimeout(2000);
  await page.getByTestId('accept-recovery').click();
  const { credentials } = await authenticator.send('WebAuthn.getCredentials', {
    authenticatorId,
  });
  const credential = credentials[0];
  await context.close();
  return {
    credentialId: credential.credentialId,
    rpId: credential.rpId,
    privateKey: credential.privateKey,
    userHandle: credential.userHandle,
  };
};
