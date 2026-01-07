import { expect } from '@playwright/test';

import { makeConnection } from './makeConnection';

export const getPasswordResetToken = async (email: string): Promise<string> => {
  const client = await makeConnection();
  const sql = `select id from "token" where email='${email}';`;
  try {
    const result = await client.query(sql);
    expect(result.rows.length).toBeGreaterThan(0);
    const token = result.rows[0]['id'];
    expect(token).toBeDefined();
    expect(token?.length).toBeGreaterThan(0);
    return token;
  } finally {
    await client.end();
  }
};
