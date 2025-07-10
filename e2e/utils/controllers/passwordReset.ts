import { Page } from 'playwright';

export const selectPasswordReset = async (page: Page) => {
  const selectButton = page.getByTestId('select-password-reset');
  selectButton.click();
};

export const setEmail = async (token: string, page: Page) => {
  await page.getByTestId('field-email').fill(token);
  await page.getByTestId('password-reset-email-submit-button').click();
};

export const setPassword = async (password: string, page: Page) => {
  await page.getByTestId('field-password').fill(password);
  await page.getByTestId('field-repeat').fill(password);
  await page.getByTestId('password-reset-submit').click();
};
