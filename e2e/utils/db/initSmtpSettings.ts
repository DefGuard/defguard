import { expect } from '@playwright/test';

import { makeConnection } from './makeConnection';

// insert settings needed for test
export const initSmtpSettings = async () => {
  const client = await makeConnection();
  const query = `
  update settings
  SET smtp_server = 'testServer',
  smtp_port = 543,
  smtp_user = 'testuser',
  smtp_password = 'test',
  smtp_sender = 'test@test.test'
  where id = 1
  returning *;
  `;
  try {
    const result = await client.query(query);
    expect(result.rows.length).toBeGreaterThan(0);
  } finally {
    await client.end();
  }
};
