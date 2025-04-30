import { Locales } from '../../i18n/i18n-types';

export const localeToDatePicker = (val: Locales): string => {
  switch (val) {
    case 'en':
      return 'en-US';
    default:
      return val;
  }
};
