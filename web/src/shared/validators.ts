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

type ParsedAclDestinationToken =
  | { type: 'network'; value: string }
  | { type: 'range'; start: ipaddr.IPv4 | ipaddr.IPv6; end: ipaddr.IPv4 | ipaddr.IPv6 };

const compareIpBytes = (left: number[], right: number[]): number => {
  const length = Math.max(left.length, right.length);

  for (let index = 0; index < length; index += 1) {
    const leftByte = left[index] ?? 0;
    const rightByte = right[index] ?? 0;

    if (leftByte !== rightByte) {
      return leftByte - rightByte;
    }
  }

  return 0;
};

const parseExactDecimalInteger = (input: string): number | null => {
  if (!/^\d+$/.test(input)) {
    return null;
  }

  const parsed = Number.parseInt(input, 10);
  if (!Number.isSafeInteger(parsed)) {
    return null;
  }

  return parsed;
};

function dottedMaskToPrefix(mask: string): number | null {
  if (!mask.includes('.')) return null;
  const maskTest =
    /^(?:255\.255\.255\.(?:0|128|192|224|240|248|252|254|255)|255\.255\.(?:0|128|192|224|240|248|252|254|255)\.0|255\.(?:0|128|192|224|240|248|252|254|255)\.0\.0|(?:0|128|192|224|240|248|252|254|255)\.0\.0\.0)$/;
  if (!maskTest.test(mask)) return null;
  const parts = mask.split('.');
  if (parts.length !== 4) return null;

  const octets: number[] = [];
  for (const part of parts) {
    const octet = parseExactDecimalInteger(part);
    if (octet === null || octet > 255) return null;

    octets.push(octet);
  }

  const binary = octets.map((part) => part.toString(2).padStart(8, '0')).join('');
  if (!/^1*0*$/.test(binary)) return null;

  return binary.indexOf('0') === -1 ? 32 : binary.indexOf('0');
}

const parseSubnetParts = (input: string): { ipPart: string; maskPart: string } | null => {
  const slashIndex = input.indexOf('/');
  if (slashIndex === -1 || slashIndex !== input.lastIndexOf('/')) {
    return null;
  }

  const ipPart = input.slice(0, slashIndex);
  const maskPart = input.slice(slashIndex + 1);
  if (ipPart === '' || maskPart === '') {
    return null;
  }

  return { ipPart, maskPart };
};

function parseSubnet(input: string): [ipaddr.IPv4 | ipaddr.IPv6, number] | null {
  const subnetParts = parseSubnetParts(input);
  if (!subnetParts) return null;

  const { ipPart, maskPart } = subnetParts;
  if (!ipaddr.isValid(ipPart)) return null;
  const ip = ipaddr.parse(ipPart);
  const kind = ip.kind();

  if (kind === 'ipv6') {
    const prefix = parseExactDecimalInteger(maskPart);
    if (prefix === null) {
      return null;
    }
    return [ip, prefix];
  }
  if (!patternStrictIpV4.test(ipPart)) return null;

  const prefix = maskPart.includes('.')
    ? dottedMaskToPrefix(maskPart)
    : parseExactDecimalInteger(maskPart);
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

const parseAclDestinationToken = (input: string): ParsedAclDestinationToken | null => {
  if (input === '') {
    return null;
  }

  const [startRaw, endRaw, ...rest] = input.split('-');
  if (endRaw === undefined) {
    return isValidIpOrCidr(input) ? { type: 'network', value: input } : null;
  }

  if (startRaw === '' || endRaw === '' || rest.length > 0) {
    return null;
  }

  if (startRaw.includes('/') || endRaw.includes('/')) {
    return null;
  }

  const start = validateIpPart(startRaw);
  const end = validateIpPart(endRaw);
  if (!start || !end || start.kind() !== end.kind()) {
    return null;
  }

  if (compareIpBytes(start.toByteArray(), end.toByteArray()) >= 0) {
    return null;
  }

  return { type: 'range', start, end };
};

const parseAclDestinations = (value: string): ParsedAclDestinationToken[] | null => {
  const normalizedValue = value.replace(/\s+/g, '');

  if (normalizedValue === '') {
    return [];
  }

  const parsedTokens: ParsedAclDestinationToken[] = [];
  for (const token of normalizedValue.split(',')) {
    const parsedToken = parseAclDestinationToken(token);
    if (!parsedToken) {
      return null;
    }

    parsedTokens.push(parsedToken);
  }

  return parsedTokens;
};

export const aclDestinationValidator = z
  .string()
  .refine(
    (value: string) => parseAclDestinations(value) !== null,
    m.form_error_invalid(),
  );
