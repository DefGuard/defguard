import { type PropsWithChildren, useEffect } from 'react';
import { themeSchema } from '../hooks/theme/types';
import { useTheme } from '../hooks/theme/useTheme';

const storageKey = 'dg-theme';

const getPreferredColorScheme = (): 'dark' | 'light' => {
  if (typeof window === 'undefined') return 'light';

  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
};

export const AppThemeProvider = ({ children }: PropsWithChildren) => {
  const { changeTheme, theme } = useTheme();

  // biome-ignore lint/correctness/useExhaustiveDependencies: on mount effect
  useEffect(() => {
    const stored = localStorage.getItem(storageKey);
    const pref = getPreferredColorScheme();
    if (!stored) {
      if (pref !== theme) {
        changeTheme(pref);
      }
    } else {
      const result = themeSchema.safeParse(stored);
      if (result.success) {
        changeTheme(result.data);
      } else {
        changeTheme(pref);
      }
    }
  }, []);

  return <>{children}</>;
};
