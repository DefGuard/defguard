import { Browser } from 'playwright';
import { waitForBase } from '../waitForBase';
import { loginBasic } from './login';
import { defaultUserAdmin, routes } from '../../config';


export const createWebhook = async (
  browser: Browser,
  url: string,
  description: string,
  secret_token?: string
): Promise<void> => {
    const context = await browser.newContext();
    const page = await context.newPage();
    await waitForBase(page);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.webhooks);
    await page.getByRole('button', { name: 'Add new' }).click();
    await page.waitForTimeout(800);
    await page.getByTestId('field-url').fill(url);
    await page.getByTestId('field-description').fill(description);
    if (secret_token){
        await page.getByTestId('field-token').fill(secret_token);
    }
    else{
        await page.getByTestId('field-token').fill("   ");
    }
    await page.getByTestId('field-on_user_created').click();
    await page.getByRole('button', { name: 'Submit' }).click();
    await page.getByRole('button', { name: 'Submit' }).click();
    await page.waitForTimeout(2000);


    await context.close();
};
