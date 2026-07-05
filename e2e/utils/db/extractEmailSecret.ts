import { expect } from '@playwright/test';
import { base32 } from '@scure/base';

import { makeConnection } from './makeConnection';

export const extractEmailSecret = async (username: string): Promise<string> => {
  const client = await makeConnection();
  const sql = 'select email_mfa_secret as secret from "user" where username = $1';
  try {
    // Poll until the secret is written: enabling email MFA stores it on the user
    // row, but the request may not be committed the instant the response returns.
    let secret: Buffer | undefined;
    for (let attempt = 0; attempt < 25; attempt++) {
      const result = await client.query(sql, [username]);
      secret = result.rows[0]?.['secret'] as Buffer | undefined;
      if (secret && secret.length > 0) break;
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
    expect(secret).toBeDefined();
    expect(secret?.length).toBeGreaterThan(0);
    const secretData = Uint8Array.from(secret as Buffer);
    const secretBase32 = base32.encode(secretData);
    return secretBase32;
  } finally {
    await client.end();
  }
};
