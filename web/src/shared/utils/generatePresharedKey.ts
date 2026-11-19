import { encode } from '@stablelib/base64';

const presharedKeyLength = 32;

export const generatePresharedKey = (): string => {
  return encode(crypto.getRandomValues(new Uint8Array(presharedKeyLength)));
};
