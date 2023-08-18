import { Page } from '@playwright/test';

export const setEnrollmentPassword = async (
  token: string,
  page: Page,
) => {
  const formElement = page.getByTestId('enrollment-token-form');
  await formElement.getByTestId('field-token').type(token);
  await formElement.locator('button[type="submit"]').click();
};
