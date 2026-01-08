import { useCallback, useState } from 'react';
import type { ThemeKey } from './types';

const storageKey = 'dg-theme';

export const useTheme = () => {
  const [theme, setTheme] = useState<ThemeKey>(
    document.documentElement.dataset.theme as ThemeKey,
  );

  const changeTheme = useCallback((newTheme: ThemeKey) => {
    document.documentElement.dataset.theme = newTheme;
    localStorage.setItem(storageKey, newTheme);
    setTheme(newTheme);
  }, []);

  return { changeTheme, theme };
};
