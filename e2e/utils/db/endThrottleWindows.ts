import { makeConnection } from './makeConnection';

export const endThrottleWindows = async (): Promise<void> => {
  const client = await makeConnection();
  try {
    await client.query("UPDATE throttle SET expires_at = expires_at - interval '1 day'");
  } finally {
    await client.end();
  }
};
