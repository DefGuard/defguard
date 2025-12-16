import { Browser } from 'playwright';

import { routes } from '../../../config';
import { User } from '../../../types';
import { waitForBase } from '../../waitForBase';
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
  await page.goto(routes.base + routes.profile);
  await page.getByTestId('passkeys-row').locator('.icon-button').click();
  await page.getByTestId('add-passkey').click();
  await page.getByTestId('field-name').fill(keyName);
  await page.getByTestId('submit').click();

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
  await page.getByTestId('confirm-code-save').click();
  await page.getByTestId('finish-recovery-codes').click();
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
