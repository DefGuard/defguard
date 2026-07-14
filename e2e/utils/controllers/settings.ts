import { Browser } from 'playwright';

import { defaultUserAdmin, routes } from '../../config';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { logout } from './logout';

// Configure SMTP in Core settings as an admin.
// Several Edge features (password reset, email MFA) are only exposed once SMTP
// is configured, so tests that exercise them must call this first.
export const setupSMTP = async (browser: Browser) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await waitForBase(page);
  await loginBasic(page, defaultUserAdmin);
  await page.goto(routes.base + routes.settings.smtp);
  await page.getByTestId('smtp-card-basic-configure').click();
  await page.getByTestId('field-smtp_server').waitFor({ state: 'visible' });
  await page.getByTestId('field-smtp_server').fill('testServer.com');
  await page.getByTestId('field-smtp_port').fill('543');
  await page.getByTestId('field-smtp_user').fill('testuser');
  await page.getByTestId('field-smtp_password').fill('test');
  await page.getByTestId('field-smtp_sender').fill('test@test.com');
  const saveButton = await page.getByTestId('submit');
  if (await saveButton.isEnabled()) {
    await saveButton.click();
  }
  await logout(page);
  await context.close();
};
