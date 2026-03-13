import ipaddr from 'ipaddr.js';

export const isValidDefguardUrl = (value: string): boolean => {
  try {
    const url = new URL(value);
    const hostname = url.hostname;

    if (hostname.length === 0) {
      return false;
    }

    if (ipaddr.isValid(hostname)) {
      return false;
    }

    return true;
  } catch {
    return false;
  }
};
