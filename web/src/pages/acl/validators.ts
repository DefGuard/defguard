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
