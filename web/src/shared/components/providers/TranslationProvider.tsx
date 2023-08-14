import { ReactNode, useEffect, useState } from 'react';
import { detectLocale, localStorageDetector } from 'typesafe-i18n/detectors';

import TypesafeI18n from '../../../i18n/i18n-react';
import { locales } from '../../../i18n/i18n-util';
import { loadLocaleAsync } from '../../../i18n/i18n-util.async';

const detectedLocale = detectLocale('en', locales, localStorageDetector);

type Props = {
  children?: ReactNode;
};

export const TranslationProvider = ({ children }: Props) => {
  const [localeLodaded, setLocaleLoaded] = useState(false);

  useEffect(() => {
    loadLocaleAsync(detectedLocale).then(() => {
      setLocaleLoaded(true);
    });
  }, []);

  if (!localeLodaded) return null;

  return <TypesafeI18n locale={detectedLocale}>{children}</TypesafeI18n>;
};
