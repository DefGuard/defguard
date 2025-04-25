import { PropsWithChildren, useEffect } from 'react';
import { navigatorDetector } from 'typesafe-i18n/detectors';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../i18n/i18n-react';
import { baseLocale, detectLocale, locales } from '../i18n/i18n-util';
import { loadLocaleAsync } from '../i18n/i18n-util.async';
import { useAppStore } from '../shared/hooks/store/useAppStore';

export const LocaleProvider = ({ children }: PropsWithChildren) => {
  const { setLocale } = useI18nContext();
  const activeLanguage = useAppStore((state) => state.language);
  const setAppStore = useAppStore((s) => s.setState, shallow);

  useEffect(() => {
    if (!activeLanguage) {
      let lang = detectLocale(navigatorDetector);
      if (!locales.includes(lang)) {
        lang = baseLocale;
      }
      setAppStore({ language: lang });
    } else {
      if (locales.includes(activeLanguage)) {
        loadLocaleAsync(activeLanguage)
          .then(() => {
            setLocale(activeLanguage);
            document.documentElement.setAttribute('lang', activeLanguage);
          })
          .catch((e) => {
            console.error(e);
          });
      } else {
        setAppStore({ language: baseLocale });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeLanguage]);

  return <>{children}</>;
};
