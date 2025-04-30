import { enUS as datePickerLocaleEnUS } from 'date-fns/locale/en-US';
import { ko as datePickerLocaleKO } from 'date-fns/locale/ko';
import { pl as datePickerLocalePL } from 'date-fns/locale/pl';
import { PropsWithChildren, useEffect, useState } from 'react';
import { registerLocale, setDefaultLocale } from 'react-datepicker';
import { navigatorDetector } from 'typesafe-i18n/detectors';
import { shallow } from 'zustand/shallow';

import TypesafeI18n from '../i18n/i18n-react';
import { baseLocale, detectLocale } from '../i18n/i18n-util';
import { loadLocale } from '../i18n/i18n-util.sync';
import { useAppStore } from '../shared/hooks/store/useAppStore';
import { localeToDatePicker } from '../shared/utils/localeToDatepicker';

// Setups i18n so useI18nContext hooks can work
export const I18nProvider = ({ children }: PropsWithChildren) => {
  const setAppState = useAppStore((s) => s.setState, shallow);
  const detectedLocale = detectLocale(navigatorDetector);
  const [localeLoaded, setLocaleLoaded] = useState(false);

  useEffect(() => {
    const lang = detectedLocale ?? baseLocale;
    loadLocale(lang);
    setLocaleLoaded(true);
    setAppState({ language: lang });
    document.documentElement.lang = lang;
    //react-datepicker
    switch (lang) {
      case 'en':
        registerLocale('en-US', datePickerLocaleEnUS);
        break;
      case 'ko':
        registerLocale('ko', datePickerLocaleKO);
        break;
      case 'pl':
        registerLocale('pl', datePickerLocalePL);
    }
    setDefaultLocale(localeToDatePicker(lang));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [detectedLocale]);

  if (!localeLoaded) return null;

  return <TypesafeI18n locale={detectedLocale}>{children}</TypesafeI18n>;
};
