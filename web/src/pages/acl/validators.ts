import * as ipaddr from 'ipaddr.js';
import { z } from 'zod';

import { TranslationFunctions } from '../../i18n/i18n-types';
import { patternStrictIpV4 } from '../../shared/patterns';

export const aclPortsValidator = (LL: TranslationFunctions) =>
  z
    .string()
    .refine((value: string) => {
      if (value === '') return true;
      const regexp = new RegExp(/^(?:\d+(?:-\d+)*)(?:(?:\s*,\s*|\s+)\d+(?:-\d+)*)*$/);
      return regexp.test(value);
    }, LL.form.error.invalid())
    .refine((value: string) => {
      if (value === '') return true;
      // check if there is no duplicates in given port field
      const trimmed = value
        .replaceAll(' ', '')
        .replaceAll('-', ' ')
        .replaceAll(',', ' ')
        .split(' ')
        .filter((v) => v !== '');
      const found: number[] = [];
      for (const entry of trimmed) {
        const num = parseInt(entry);
        if (isNaN(num)) {
          return false;
        }
        if (found.includes(num)) {
          return false;
        }
        found.push(num);
      }
      return true;
    }, LL.form.error.invalid())
    .refine((value: string) => {
      if (value === '') return true;
      // check if ranges in input are valid means follow pattern <start>-<end>
      const matches = value.match(/\b\d+-\d+\b/g);
      if (Array.isArray(matches)) {
        for (const match of matches) {
          const split = match.split('-');
          if (split.length !== 2) {
            return false;
          }
          const start = split[0];
          const end = split[1];
          if (start >= end) {
            return false;
          }
        }
      }
      return true;
    }, LL.form.error.invalid());

function dottedMaskToPrefix(mask: string): number | null {
  if (!mask.includes('.')) return Number(mask);
  if (mask.split('.').length !== 4) return null;
  const parts = mask.split('.').map(Number);
  if (parts.length !== 4 || parts.some((part) => part < 0 || part > 255)) return null;

  const binary = parts.map((part) => part.toString(2).padStart(8, '0')).join('');
  if (!/^1*0*$/.test(binary)) return null;

  return binary.indexOf('0') === -1 ? 32 : binary.indexOf('0');
}

const validateIpPart = (input: string): ipaddr.IPv4 | ipaddr.IPv6 | null => {
  if (!ipaddr.isValid(input)) return null;
  const ip = ipaddr.parse(input);
  if (ip.kind() === 'ipv6') {
    return ip;
  }
  if (!patternStrictIpV4.test(input)) return null;
  return ip;
};

function parseSubnet(input: string): [ipaddr.IPv4 | ipaddr.IPv6, number] | null {
  const [ipPart, maskPart] = input.split('/');
  if (!ipaddr.isValid(ipPart) || !maskPart) return null;
  const ip = ipaddr.parse(ipPart);
  const kind = ip.kind();

  if (kind === 'ipv6') {
    const prefix = parseInt(maskPart);
    if (typeof prefix !== 'number' || isNaN(prefix)) {
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

export const aclDestinationValidator = (LL: TranslationFunctions) =>
  z.string().refine((value: string) => {
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
  }, LL.form.error.invalid());
