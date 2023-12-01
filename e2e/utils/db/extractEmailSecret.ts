import { expect } from '@playwright/test';
import { base32 } from '@scure/base';

import { makeConnection } from './makeConnection';

export const extractEmailSecret = async (username: string): Promise<string> => {
  const client = await makeConnection();
  const sql = `select email_mfa_secret as secret from "user" where username='${username}';`;
  try {
    const result = await client.query(sql);
    expect(result.rows.length).toBeGreaterThan(0);
    const secret = result.rows[0]['secret'] as Buffer;
    expect(secret).toBeDefined();
    expect(secret?.length).toBeGreaterThan(0);
    const secretData = Uint8Array.from(secret);
    const secretBase32 = base32.encode(secretData);
    return secretBase32;
  } finally {
    await client.end();
  }
};
