import ipaddr from 'ipaddr.js';

import { patternValidDomain } from './patterns';

// Returns flase when invalid
export const validateIpOrDomain = (
  val: string,
  allowMask = false,
  allowIPv6 = false,
): boolean => {
  return (
    (allowIPv6 && validateIPv6(val, allowMask)) ||
    validateIPv4(val, allowMask) ||
    patternValidDomain.test(val)
  );
};

// Returns flase when invalid
export const validateIpList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
): boolean => {
  const trimed = val.replace(' ', '');
  const split = trimed.split(splitWith);
  for (const value of split) {
    if (!validateIPv4(value, allowMasks)) {
      return false;
    }
  }
  return true;
};

// Returns flase when invalid
export const validateIpOrDomainList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
  allowIPv6 = false,
): boolean => {
  const trimmed = val.replace(' ', '');
  const split = trimmed.split(splitWith);
  for (const value of split) {
    if (
      !validateIPv4(value, allowMasks) &&
      !patternValidDomain.test(value) &&
      (!allowIPv6 || !validateIPv6(value, allowMasks))
    ) {
      return false;
    }
  }
  return true;
};

// Returns false when invalid
export const validateIPv4 = (ip: string, allowMask = false): boolean => {
  if (allowMask) {
    if (ip.includes('/')) {
      ipaddr.IPv4.isValidCIDR(ip);
    }
  }
  return ipaddr.IPv4.isValid(ip);
};

export const validateIPv6 = (ip: string, allowMask = false): boolean => {
  if (allowMask) {
    if (ip.includes('/')) {
      ipaddr.IPv6.isValidCIDR(ip);
    }
  }
  return ipaddr.IPv6.isValid(ip);
};

export const validatePort = (val: string) => {
  const parsed = parseInt(val);
  if (!isNaN(parsed)) {
    return parsed <= 65535;
  }
};

export const numericString = (val: string) => /^\d+$/.test(val);

export const numericStringFloat = (val: string) => /^\d*\.?\d+$/.test(val);
