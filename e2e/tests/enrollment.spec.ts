import { BrowserContext, Page, expect, test } from '@playwright/test';
import { dockerRestart } from '../utils/docker';
import { waitForBase } from '../utils/waitForBase';
import { createUserEnrollment } from '../utils/controllers/createUserEnrollment';
import { setEnrollmentPassword } from '../utils/controllers/setEnrollmentPassword';
import { getPageClipboard } from '../utils/getPageClipboard';
import { waitForPromise } from '../utils/waitForPromise';
import { logout } from '../utils/controllers/logout';

//test.describe.configure({
  //mode: 'serial',
//});

test.describe('Create user with enrollment enabled', () => {
  let token: string;
  let url: string;
  let page: Page;
  let context: BrowserContext;

  // Setup client and user for tests
  test.beforeAll(async ({ browser }) => {
    context = await browser.newContext();
    page = await context.newPage();
    await waitForBase(page);
    await createUserEnrollment(context, 'testauth01');
    const response = (await getPageClipboard(page)).split('\n');
    // Extract token and url
    url = response[0].split(' ')[2];
    const tokenResponse = response[1].split(' ')[1];
    token = tokenResponse;
  });

  test.afterAll(() => {
    dockerRestart();
  });

  test('Go to enrollment', async () => {
    expect(token).toBeDefined();
    expect(url).toBeDefined();
    await page.waitForURL(url);
    await waitForPromise(2000);
    await setEnrollmentPassword(token, page);
  });
});
