import ipaddr from 'ipaddr.js';
import { z } from 'zod';
import {
  domainPattern,
  ipv4Pattern,
  ipv4WithCIDRPattern,
  ipv4WithPortPattern,
  patternValidWireguardKey,
} from './patterns';

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

export const Validate = {
  IPv4: (ip: string): boolean => {
    if (!ipv4Pattern.test(ip)) {
      return false;
    }
    if (!ipaddr.IPv4.isValid(ip)) {
      return false;
    }
    return true;
  },
  IPv4withPort: (ip: string): boolean => {
    if (!ipv4WithPortPattern.test(ip)) {
      return false;
    }
    const addr = ip.split(':');
    if (!ipaddr.IPv4.isValid(addr[0]) || !Validate.Port(addr[1])) {
      return false;
    }
    return true;
  },
  IPv6: (ip: string): boolean => {
    if (!ipaddr.IPv6.isValid(ip)) {
      return false;
    }
    return true;
  },
  IPv6withPort: (ip: string): boolean => {
    if (ip.includes(']')) {
      const address = ip.split(']');
      const ipv6 = address[0].replaceAll('[', '').replaceAll(']', '');
      const port = address[1].replaceAll(']', '').replaceAll(':', '');
      if (!ipaddr.IPv6.isValid(ipv6)) {
        return false;
      }
      if (!Validate.Port(port)) {
        return false;
      }
    } else {
      return false;
    }
    return true;
  },
  CIDRv4: (ip: string): boolean => {
    if (!ipv4WithCIDRPattern.test(ip)) {
      return false;
    }
    if (ip.endsWith('/0')) {
      return false;
    }
    if (!ipaddr.IPv4.isValidCIDR(ip)) {
      return false;
    }
    return true;
  },
  CIDRv6: (ip: string): boolean => {
    if (ip.endsWith('/0')) {
      return false;
    }
    if (!ipaddr.IPv6.isValidCIDR(ip)) {
      return false;
    }
    return true;
  },
  Domain: (ip: string): boolean => {
    if (!domainPattern.test(ip)) {
      return false;
    }
    return true;
  },
  DomainWithPort: (ip: string): boolean => {
    const splitted = ip.split(':');
    const domain = splitted[0];
    const port = splitted[1];
    if (!Validate.Port(port)) {
      return false;
    }
    if (!domainPattern.test(domain)) {
      return false;
    }
    return true;
  },
  Port: (val: string): boolean => {
    const parsed = Number(val);
    if (Number.isNaN(parsed) || !Number.isInteger(parsed)) {
      return false;
    }
    return 0 < parsed && parsed <= 65535;
  },
  Empty: (val: string): boolean => {
    if (val === '' || !val) {
      return true;
    }
    return false;
  },
  any: (
    value: string | undefined,
    validators: Array<(val: string) => boolean>,
    max: number = 0,
    splitWith = ',',
  ): boolean => {
    if (!value) {
      return true;
    }
    const items = value.replaceAll(' ', '').split(splitWith);

    if (max !== 0 && items.length > max) {
      return false;
    }

    for (const item of items) {
      let valid = false;
      for (const validator of validators) {
        if (validator(item)) {
          valid = true;
          break;
        }
      }
      if (!valid) {
        return false;
      }
    }

    return true;
  },
  all: (
    value: string | undefined,
    validators: Array<(val: string) => boolean>,
    max: number = 0,
    splitWith = ',',
  ): boolean => {
    if (!value) {
      return true;
    }
    const items = value.replaceAll(' ', '').split(splitWith);

    if (max !== 0 && items.length > max) {
      return false;
    }

    for (const item of items) {
      for (const validator of validators) {
        if (!validator(item)) {
          return false;
        }
      }
    }

    return true;
  },
} as const;

export const numericString = (val: string) => /^\d+$/.test(val);

export const numericStringFloat = (val: string) => /^\d*\.?\d+$/.test(val);
