import { ThemeKey } from '../shared/components/layout/hooks/theme/types';
import { isThemeKey } from '../shared/components/layout/hooks/theme/utils';

/*Sets initial data-theme on html element*/
export const initTheme = () => {
  const currentTheme: ThemeKey = document.documentElement.dataset.theme as ThemeKey;
  const darkTheme: ThemeKey = 'dark';
  const lightTheme: ThemeKey = 'light';
  const isDarkModePreferred = window.matchMedia('(prefers-color-scheme: dark)');

  if (!isThemeKey(currentTheme)) {
    console.error(`Currently set theme (${currentTheme}) is not exisiting.`);
  }

  if (isDarkModePreferred && currentTheme === 'light') {
    document.documentElement.dataset.theme = darkTheme;
  }

  if (!isDarkModePreferred && currentTheme === 'dark') {
    document.documentElement.dataset.theme = lightTheme;
  }
};
