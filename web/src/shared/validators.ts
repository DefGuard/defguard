import ipaddr from 'ipaddr.js';
import { z } from 'zod';
import { m } from '../paraglide/messages';
import { patternStrictIpV4, patternValidWireguardKey } from './patterns';
import { Validate } from './validate';

export const validateWireguardPublicKey = () =>
  z
    .string(m.form_error_required())
    .length(
      44,
      m.form_error_len({
        length: 44,
      }),
    )
    .regex(patternValidWireguardKey, m.form_error_invalid());

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
    return Validate.Domain(val);
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
      !Validate.Domain(value) &&
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

export const validatePort = (val: string): boolean => {
  return parsePortNumber(val) !== null;
};

type ParsedAclPortToken = [number] | [number, number];

const normalizeAclPortsInput = (value: string): string => value.replace(/\s+/g, '');

const parsePortNumber = (value: string): number | null => {
  if (!/^\d+$/.test(value)) {
    return null;
  }

  const parsed = Number.parseInt(value, 10);
  if (Number.isNaN(parsed) || parsed < 0 || parsed > 65535) {
    return null;
  }

  return parsed;
};

const parseAclPortToken = (value: string): ParsedAclPortToken | null => {
  if (value === '') {
    return null;
  }

  const [startRaw, endRaw, ...rest] = value.split('-');
  const start = parsePortNumber(startRaw);

  if (start === null) {
    return null;
  }

  if (endRaw === undefined) {
    return [start];
  }

  if (endRaw === '' || rest.length > 0) {
    return null;
  }

  const end = parsePortNumber(endRaw);
  if (end === null || start >= end) {
    return null;
  }

  return [start, end];
};

const parseAclPorts = (value: string): ParsedAclPortToken[] | null => {
  const normalizedValue = normalizeAclPortsInput(value);

  if (normalizedValue === '') {
    return [];
  }

  const parsedTokens: ParsedAclPortToken[] = [];
  for (const token of normalizedValue.split(',')) {
    const parsedToken = parseAclPortToken(token);
    if (!parsedToken) {
      return null;
    }

    parsedTokens.push(parsedToken);
  }

  return parsedTokens;
};

export const numericString = (val: string) => /^\d+$/.test(val);

export const numericStringFloat = (val: string) => /^\d*\.?\d+$/.test(val);

export const aclPortsValidator = z
  .string()
  .refine((value: string) => parseAclPorts(value) !== null, m.form_error_invalid());

const validateIpPart = (input: string): ipaddr.IPv4 | ipaddr.IPv6 | null => {
  if (!ipaddr.isValid(input)) return null;
  const ip = ipaddr.parse(input);
  if (ip.kind() === 'ipv6') {
    return ip;
  }
  if (!patternStrictIpV4.test(input)) return null;
  return ip;
};

function dottedMaskToPrefix(mask: string): number | null {
  if (!mask.includes('.')) return Number(mask);
  const maskTest =
    /^(?:255\.255\.255\.(?:0|128|192|224|240|248|252|254|255)|255\.255\.(?:0|128|192|224|240|248|252|254|255)\.0|255\.(?:0|128|192|224|240|248|252|254|255)\.0\.0|(?:0|128|192|224|240|248|252|254|255)\.0\.0\.0)$/;
  if (!maskTest.test(mask)) return null;
  if (mask.split('.').length !== 4) return null;
  const parts = mask.split('.').map(Number);
  if (parts.length !== 4 || parts.some((part) => part < 0 || part > 255)) return null;

  const binary = parts.map((part) => part.toString(2).padStart(8, '0')).join('');
  if (!/^1*0*$/.test(binary)) return null;

  return binary.indexOf('0') === -1 ? 32 : binary.indexOf('0');
}

function parseSubnet(input: string): [ipaddr.IPv4 | ipaddr.IPv6, number] | null {
  const [ipPart, maskPart] = input.split('/');
  if (!ipaddr.isValid(ipPart) || !maskPart) return null;
  const ip = ipaddr.parse(ipPart);
  const kind = ip.kind();

  if (kind === 'ipv6') {
    const prefix = parseInt(maskPart, 10);
    if (typeof prefix !== 'number' || Number.isNaN(prefix)) {
      return null;
    }
    return [ip, prefix];
  }
  if (!patternStrictIpV4.test(ipPart)) return null;

  const prefix = dottedMaskToPrefix(maskPart);
  if (prefix === null) return null;

  return [ip, prefix];
}

function isValidIpOrCidr(input: string): boolean {
  try {
    if (input.includes('/')) {
      const parsed = parseSubnet(input);
      if (!parsed) return false;
      const [ip, mask] = parsed;
      const cidr = ipaddr.parseCIDR(`${ip.toString()}/${mask}`);
      return cidr[0] !== undefined && typeof cidr[1] === 'number';
    } else {
      return validateIpPart(input) !== null;
    }
  } catch {
    return false;
  }
}

export const aclDestinationValidator = z.string().refine((value: string) => {
  if (value === '') return true;

  const entries = value.split(',').map((s) => s.trim());

  for (const entry of entries) {
    if (entry.includes('-')) {
      const [start, end] = entry.split('-').map((s) => s.trim());

      // reject CIDR notation used in ranges
      if (start.includes('/') || end.includes('/')) return false;

      if (!ipaddr.isValid(start) || !ipaddr.isValid(end)) return false;

      const startAddr = ipaddr.parse(start);
      const endAddr = ipaddr.parse(end);

      // reject different ip versions in ranges
      if (startAddr.kind() !== endAddr.kind()) return false;

      // reject invalid order in ranges
      if (startAddr.toByteArray().join('.') > endAddr.toByteArray().join('.')) {
        return false;
      }
    } else {
      if (!isValidIpOrCidr(entry)) return false;
    }
  }

  return true;
}, m.form_error_invalid());
