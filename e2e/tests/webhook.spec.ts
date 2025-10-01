import { expect, test } from '@playwright/test';
import { dockerRestart } from '../utils/docker';
import { defaultUserAdmin, routes } from '../config';
import { createWebhook } from '../utils/controllers/webhooks';
import { loginBasic } from '../utils/controllers/login';

test.describe('Test webhooks', () => {
  test.beforeEach(async ({ page }) => {
    dockerRestart();
  });
  const webhook_url ="https://defguard.defguard/webhook";
  const webhook_description="example webhook";
  const webhook_secret="secret";


  test('Create webhook and verify content', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, webhook_secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.webhooks, {
      waitUntil: 'networkidle'
    });
    await page.waitForTimeout(2000);
    
    const webhookRow = page.locator('.default-row');
    
    const webhook_url_cell = await webhookRow.locator('.cell-0 span').textContent();
    expect(webhook_url_cell).toBe(webhook_url);
    
    const webhook_description_cell = await webhookRow.locator('.cell-1 span').textContent();
    expect(webhook_description_cell).toBe(webhook_description); 
    
    const webhook_state_cell = await webhookRow.locator('.cell-2 span').textContent();
    expect(webhook_state_cell).toBe('Enabled');
    
    const editButton = webhookRow.locator('.cell-3 .edit-button');
    await expect(editButton).toBeVisible();
  });


  const new_webhook_url ="https://changed.defguard/webhook";
  const new_webhook_description="changed webhook";
  test('Create, modify webhook and verify content', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, "secret");
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.webhooks, {
      waitUntil: 'networkidle'
    });
    await page.waitForTimeout(2000);

    const webhookRow = page.locator('.default-row');
    
    const webhook_url_cell = await webhookRow.locator('.cell-0 span').textContent();
    expect(webhook_url_cell).toBe(webhook_url);
    
    const webhook_description_cell = await webhookRow.locator('.cell-1 span').textContent();
    expect(webhook_description_cell).toBe(webhook_description); 
    
    const webhook_state_cell = await webhookRow.locator('.cell-2 span').textContent();
    expect(webhook_state_cell).toBe('Enabled');
    
    const editButton = webhookRow.locator('.cell-3 .edit-button');
    await expect(editButton).toBeVisible();
    // check if webhook is OK 
    // then edit webhook
    await webhookRow.locator('.cell-3 .edit-button').click();

    await page.locator('.edit-button-floating-ui').getByRole('button', { name: 'Edit' }).click();

    await page.getByTestId('field-url').fill(new_webhook_url);
    await page.getByTestId('field-description').fill(new_webhook_description);
    await page.getByRole('button', { name: 'Submit' }).click();
    await page.waitForTimeout(1000);
      
    const changed_webhookRow = page.locator('.default-row');    
    const changed_webhook_url_cell = await changed_webhookRow.locator('.cell-0 span').textContent();
    expect(changed_webhook_url_cell).toBe(new_webhook_url);
    
    const changed_webhook_description_cell = await changed_webhookRow.locator('.cell-1 span').textContent();
    expect(changed_webhook_description_cell).toBe(new_webhook_description); 
    
    const changed_webhook_state_cell = await changed_webhookRow.locator('.cell-2 span').textContent();
    expect(changed_webhook_state_cell).toBe('Enabled');
    
  });

  test('Create webhook, change state and verify content', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, webhook_secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.webhooks, {
      waitUntil: 'networkidle'
    });
    await page.waitForTimeout(2000);
    const webhookRow = page.locator('.default-row');
    await webhookRow.locator('.cell-3 .edit-button').click();
    await page.locator('.edit-button-floating-ui').getByRole('button', { name: 'Disable' }).click();
    await page.waitForTimeout(2000);



    // is everything ok after changing state to Disabled?
    const webhook_url_cell = await webhookRow.locator('.cell-0 span').textContent();
    expect(webhook_url_cell).toBe(webhook_url);
    
    const webhook_description_cell = await webhookRow.locator('.cell-1 span').textContent();
    expect(webhook_description_cell).toBe(webhook_description); 
    
    const webhook_state_cell = await webhookRow.locator('.cell-2 span').textContent();
    expect(webhook_state_cell).toBe('Disabled');
    
    const editButton = webhookRow.locator('.cell-3 .edit-button');
    await expect(editButton).toBeVisible();

    await webhookRow.locator('.cell-3 .edit-button').click();
    await page.locator('.edit-button-floating-ui').getByRole('button', { name: 'Enable' }).click();
    await page.waitForTimeout(2000);

    // is everything ok after changing state to Enabled?
    const changed_webhook_url_cell = await webhookRow.locator('.cell-0 span').textContent();
    expect(changed_webhook_url_cell).toBe(webhook_url);
    
    const changed_webhook_description_cell = await webhookRow.locator('.cell-1 span').textContent();
    expect(changed_webhook_description_cell).toBe(webhook_description); 
    
    const changed_webhook_state_cell = await webhookRow.locator('.cell-2 span').textContent();
    expect(changed_webhook_state_cell).toBe('Enabled');
    
    const changed_editButton = webhookRow.locator('.cell-3 .edit-button');
    await expect(changed_editButton).toBeVisible();
  });
  test('Create webhook and delete it', async ({ page, browser }) => {
    await createWebhook(browser, webhook_url, webhook_description, webhook_secret);
    await loginBasic(page, defaultUserAdmin);
    await page.goto(routes.base + routes.admin.webhooks, {
      waitUntil: 'networkidle'
    });
    await page.waitForTimeout(2000);

    const webhookRow = page.locator('.default-row');
    const editButton = webhookRow.locator('.cell-3 .edit-button');
    await expect(editButton).toBeVisible();
    // check if webhook is OK 
    // then edit webhook
    await webhookRow.locator('.cell-3 .edit-button').click();
    await page.locator('.edit-button-floating-ui').getByRole('button', { name: 'Delete webhook' }).click();
    await page.locator('.modal-content').waitFor({ state: 'visible', timeout: 5000 });  
    await page.getByRole('button', { name: 'Delete' }).click();
    await page.waitForTimeout(2000);
    const webhookRows = page.locator('.default-row');
    await expect(webhookRows).toHaveCount(0);

  });
});