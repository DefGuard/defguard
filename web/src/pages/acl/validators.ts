import * as ipaddr from 'ipaddr.js';
import { z } from 'zod';

import { TranslationFunctions } from '../../i18n/i18n-types';

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

function isValidIpOrCidr(input: string): boolean {
  try {
    if (input.includes('/')) {
      const [ip, mask] = ipaddr.parseCIDR(input);
      return ip !== undefined && typeof mask === 'number';
    } else {
      return ipaddr.isValid(input);
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
