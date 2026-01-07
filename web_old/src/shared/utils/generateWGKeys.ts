import { encode } from '@stablelib/base64';
import { generateKeyPair } from '@stablelib/x25519';

export const generateWGKeys = () => {
  const keys = generateKeyPair();
  const publicKey = encode(keys.publicKey);
  const privateKey = encode(keys.secretKey);
  return { publicKey, privateKey };
};
