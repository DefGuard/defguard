import ipaddr from 'ipaddr.js';
import {
  domainPattern,
  hostnamePattern,
  ipv4Pattern,
  ipv4WithCIDRPattern,
  ipv4WithPortPattern,
} from './patterns';

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
  CIDRv4: (ip: string, allow_zero: boolean = false): boolean => {
    if (!ipv4WithCIDRPattern.test(ip)) {
      return false;
    }
    const ipPart = ip.split('/')[0];
    if (ipPart.split('.').some((octet) => octet.length > 1 && octet.startsWith('0'))) {
      return false;
    }
    if (ip.endsWith('/0') && !allow_zero) {
      return false;
    }
    if (!ipaddr.IPv4.isValidCIDR(ip)) {
      return false;
    }
    return true;
  },
  CIDRv6: (ip: string, allow_zero: boolean = false): boolean => {
    if (ip.endsWith('/0') && !allow_zero) {
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
  // Single-label hostname e.g. "localhost"
  Hostname: (hostname: string): boolean => {
    return hostnamePattern.test(hostname);
  },
  any: (
    value: string | undefined,
    validators: Array<(val: string) => boolean>,
    allowList: boolean = false,
    splitWith = ',',
  ): boolean => {
    if (!value) {
      return true;
    }
    const items = value.split(splitWith).map((item) => item.trim());

    if (items.length > 1 && !allowList) {
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
    allowList: boolean = false,
    splitWith = ',',
  ): boolean => {
    if (!value) {
      return true;
    }
    const items = value.split(splitWith).map((item) => item.trim());

    if (items.length > 1 && !allowList) {
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
  isNetworkAddress: (cidr: string): boolean => {
    try {
      if (ipaddr.IPv4.isValidCIDR(cidr)) {
        const [addr] = ipaddr.parseCIDR(cidr);
        const network = ipaddr.IPv4.networkAddressFromCIDR(cidr);
        return addr.toString() === network.toString();
      }
      if (ipaddr.IPv6.isValidCIDR(cidr)) {
        const [addr] = ipaddr.parseCIDR(cidr);
        const network = ipaddr.IPv6.networkAddressFromCIDR(cidr);
        return addr.toString() === network.toString();
      }
      return false;
    } catch {
      return false;
    }
  },
  isBroadcastAddress: (cidr: string): boolean => {
    try {
      if (!ipaddr.IPv4.isValidCIDR(cidr)) return false;
      const [addr, prefixLen] = ipaddr.parseCIDR(cidr);
      // /31 and /32 have no broadcast address
      if (prefixLen >= 31) return false;
      return addr.toString() === ipaddr.IPv4.broadcastAddressFromCIDR(cidr).toString();
    } catch {
      return false;
    }
  },
} as const;
