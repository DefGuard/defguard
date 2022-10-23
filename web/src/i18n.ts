import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import en from './locales/en/translation.json';

export const resources = {
  en: {
    en: en,
  },
} as const;

const defaultNS = 'en';

i18n.use(initReactI18next).init({
  defaultNS,
  ns: ['en'],
  resources,
  lng: 'en',
  fallbackLng: 'en',
  interpolation: {
    escapeValue: false,
  },
  react: {
    useSuspense: false,
  },
});

export default i18n;
