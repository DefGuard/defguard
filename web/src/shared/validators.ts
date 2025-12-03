import ipaddr from 'ipaddr.js';
import { z } from 'zod';
import { patternValidDomain, patternValidWireguardKey } from './patterns';

export const validateWireguardPublicKey = (props: {
  requiredError: string;
  minError: string;
  maxError: string;
  validKeyError: string;
}) =>
  z
    .string({
      invalid_type_error: props.requiredError,
      required_error: props.requiredError,
    })
    .min(44, props.minError)
    .max(44, props.maxError)
    .regex(patternValidWireguardKey, props.validKeyError);

// Returns false when invalid
export const validateIpOrDomain = (
  val: string,
  allowMask = false,
  allowIPv6 = false,
): boolean => {
  const hasLetter = /\p{L}/u.test(val);
  const hasColon = /:/.test(val);
  if (!hasLetter || hasColon) {
    return (allowIPv6 && validateIPv6(val, allowMask)) || validateIPv4(val, allowMask);
  } else {
    return patternValidDomain.test(val);
  }
};

// Returns false when invalid
export const validateIpList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
): boolean => {
  return val
    .replace(' ', '')
    .split(splitWith)
    .every((el) => {
      if (!el.includes('/') && allowMasks) return false;
      return validateIPv4(el, allowMasks) || validateIPv6(el, allowMasks);
    });
};

// Returns false when invalid
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
    if (ip.endsWith('/0')) {
      return false;
    }
    if (ip.includes('/')) {
      return ipaddr.IPv4.isValidCIDR(ip);
    }
  }
  const ipv4Pattern = /^(\d{1,3}\.){3}\d{1,3}$/;
  const ipv4WithPortPattern = /^(\d{1,3}\.){3}\d{1,3}:\d{1,5}$/;
  if (!ipv4Pattern.test(ip) && !ipv4WithPortPattern.test(ip)) {
    return false;
  }

  if (ipv4WithPortPattern.test(ip)) {
    const [address, port] = ip.split(':');
    ip = address;
    if (!validatePort(port)) {
      return false;
    }
  }

  return ipaddr.IPv4.isValid(ip);
};

export const validateIPv6 = (ip: string, allowMask = false): boolean => {
  if (allowMask) {
    if (ip.endsWith('/0')) {
      return false;
    }
    if (ip.includes('/')) {
      return ipaddr.IPv6.isValidCIDR(ip);
    }
  }
  return ipaddr.IPv6.isValid(ip);
};

export const validatePort = (val: string) => {
  const parsed = parseInt(val, 10);
  if (!Number.isNaN(parsed)) {
    return parsed <= 65535;
  }
};

export const numericString = (val: string) => /^\d+$/.test(val);

export const numericStringFloat = (val: string) => /^\d*\.?\d+$/.test(val);
