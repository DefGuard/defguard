import { useCallback, useEffect, useMemo, useState } from 'react';

import { themeDark, ThemeKey, themeLight } from './types';

export const useTheme = () => {
  const [theme, setTheme] = useState<ThemeKey>(
    document.documentElement.dataset.theme as ThemeKey
  );

  const changeTheme = useCallback((newTheme: string) => {
    document.documentElement.dataset.theme = newTheme;
  }, []);

  const colors = useMemo(() => {
    switch (theme) {
      case 'dark':
        return themeDark;
      case 'light':
        return themeLight;
      default:
        return themeLight;
    }
  }, [theme]);

  useEffect(() => {
    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'attributes' && mutation.attributeName === 'data-theme') {
          setTheme(document.documentElement.dataset.theme as ThemeKey);
        }
      }
    });

    observer.observe(document.documentElement, { attributes: true });

    return () => {
      observer.disconnect();
    };
  }, []);

  return { changeTheme, theme, colors };
};
