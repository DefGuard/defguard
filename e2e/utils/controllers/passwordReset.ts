import { Page } from 'playwright';

export const selectPasswordReset = async (page: Page) => {
  const selectButton = page.getByTestId('start-password-reset');
  await selectButton.waitFor({ state: 'visible' });
  await selectButton.click();
};

export const setEmail = async (email: string, page: Page) => {
  await page.getByTestId('field-email').waitFor({ state: 'visible' });
  await page.getByTestId('field-email').fill(email);
  await page.getByTestId('page-nav-next').click();
  // Wait for the email step to complete (field hidden = server processed the request).
  await page.getByTestId('field-email').waitFor({ state: 'hidden' });
};

export const setPassword = async (password: string, page: Page) => {
  await page.getByTestId('field-password').fill(password);
  await page.getByTestId('field-repeat').fill(password);
  await page.getByTestId('form-submit').click();
};
