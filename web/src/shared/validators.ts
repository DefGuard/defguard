import ipaddr from 'ipaddr.js';
import { z } from 'zod';
import {
  domainPattern,
  domainWithPortPattern,
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
  IPv4: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      if (!ipv4Pattern.test(ip)) {
        return false;
      }
      if (!ipaddr.IPv4.isValid(ip)) {
        return false;
      }
    }
    return true;
  },
  IPv4withPort: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      if (!ipv4WithPortPattern.test(ip)) {
        return false;
      }
      const addr = ip.split(':');
      if (!ipaddr.IPv4.isValid(addr[0]) || !Validate.Port(addr[1])) {
        return false;
      }
    }
    return true;
  },
  IPv6: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      if (!ipaddr.IPv6.isValid(ip)) {
        return false;
      }
    }
    return true;
  },
  IPv6withPort: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      if (ip.includes(']')) {
        const address = ip.split(']');
        const ipv6 = address[0];
        const port = address[1];
        if (!ipaddr.IPv6.isValid(ipv6)) {
          return false;
        }
        if (!Validate.Port(port)) {
          return false;
        }
      }
      return false;
    }
    return true;
  },
  CIDRv4: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      if (!ipv4WithCIDRPattern.test(ip)) {
        return false;
      }
      if (ip.endsWith('/0')) {
        return false;
      }
      if (!ipaddr.IPv4.isValidCIDR(ip)) {
        return false;
      }
    }
    return true;
  },
  CIDRv6: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      if (ip.endsWith('/0')) {
        return false;
      }
      if (!ipaddr.IPv6.isValidCIDR(ip)) {
        return false;
      }
    }
    return true;
  },
  Domain: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      if (!domainPattern.test(ip)) {
        return false;
      }
    }
    return true;
  },
  DomainWithPort: (address: string, splitWith = ','): boolean => {
    for (const ip of address.replace(' ', '').split(splitWith)) {
      const splitted = ip.split(':');
      const domain = splitted[0];
      const port = splitted[1];
      console.log(domainWithPortPattern.test(domain));

      if (!Validate.Port(port)) {
        return false;
      }
      if (!domainPattern.test(domain)) {
        return false;
      }
    }
    return true;
  },
  Port: (val: string): boolean => {
    const parsed = parseInt(val, 10);
    if (!Number.isNaN(parsed)) {
      return parsed <= 65535;
    } else {
      return false;
    }
  },
} as const;

export const numericString = (val: string) => /^\d+$/.test(val);

export const numericStringFloat = (val: string) => /^\d*\.?\d+$/.test(val);
