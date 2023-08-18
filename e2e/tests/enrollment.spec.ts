import { BrowserContext, Page, expect, test } from '@playwright/test';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { setPassword, setToken, createUserEnrollment, validateData, password } from '../utils/controllers/enrollment';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForPromise } from '../utils/waitForPromise';
import { User } from '../types';
import { loginBasic } from '../utils/controllers/login';

test.afterEach(async () => {
  dockerRestart();
});

test.describe.configure({
  mode: 'serial',
});

test.describe('Create user with enrollment enabled', () => {
  let token: string;
  let page: Page;
  let context: BrowserContext;
  let user: User;

  // Setup client and user for tests
  test.beforeAll(async ({ browser }) => {
    context = await browser.newContext();
    page = await context.newPage();
    await waitForBase(page);
    user = await createUserEnrollment(context, 'testauth01');
    const response = (await getPageClipboard(page)).split('\n');
    // Extract token and url
    const tokenResponse = response[1].split(' ')[1];
    token = tokenResponse;
  });

  test.afterAll(() => {
    dockerRestart();
  });

  test('Go to enrollment', async () => {
    expect(token).toBeDefined();
    await page.goto('http://localhost:8080/');
    await waitForPromise(2000);
    await setToken(page, token);
    // Welcome page
    await page.getByTestId('enrollment-next').click();
    // Data validation
    await validateData(page, user);
    await page.getByTestId('enrollment-next').click();
    // Set password
    await setPassword(page);
    // VPN 
    await page.getByTestId('enrollment-next').click();
    // Finish message
    await page.getByTestId('enrollment-next').click();
    loginBasic(page, { username: 'testauth01', password });
    await waitForPromise(2000);
  });
});
