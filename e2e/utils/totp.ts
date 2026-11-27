import { setTimeout } from 'node:timers/promises';

import { TOTP } from 'totp-generator';

export const generateFreshTotp = async (secret: string): Promise<string> => {
  const { otp, expires } = await TOTP.generate(secret);
  if (expires - Date.now() > 3_000) return otp;
  await setTimeout(expires - Date.now());
  return (await TOTP.generate(secret)).otp;
};
