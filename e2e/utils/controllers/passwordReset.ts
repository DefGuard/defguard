import { Page } from "playwright";


export const selectPasswordReset = async (page: Page) => {
  const selectButton = page.getByTestId('select-password-reset');
  selectButton.click();
};

export const setEmail = async (token: string, page: Page) => {
  await page.getByTestId('field-email').type(token);
  await page.getByTestId('password-reset-email-submit-button').click();
};

export const setPassword = async (password: string, page: Page) => {
  await page.getByTestId('field-password').type(password);
  await page.getByTestId('field-repeat').type(password);
  await page.getByTestId('password-reset-submit').click();
};
