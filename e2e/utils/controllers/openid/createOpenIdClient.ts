import { Page } from 'playwright';

import { routes } from '../../../config';
import { OpenIdClient } from '../../../types';

export const CreateOpenIdClient = async (page: Page, client: OpenIdClient) => {
  await page.goto(routes.base + routes.admin.openid, { waitUntil: 'networkidle' });
  await page.getByTestId('add-openid-client').click();
  const modalElement = page.locator('#openid-client-modal');
  await modalElement.waitFor({ state: 'visible' });
  const modalForm = modalElement.locator('form');
  await modalForm.getByTestId('field-name').type(client.name);
  await modalForm.getByTestId('field-redirect_uri.0.url').type(client.redirectURL);
  for (const scope of client.scopes) {
    await modalForm.getByTestId(`field-scope-${scope}`).click();
  }
  await modalForm.locator('button[type="submit"]').click();
  await modalElement.waitFor({ state: 'hidden' });
};
