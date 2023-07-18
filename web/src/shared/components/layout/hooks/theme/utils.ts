import { avaliableThemes, ThemeKey } from './types';

export const isThemeKey = (val: string): val is ThemeKey =>
  avaliableThemes.includes(val as ThemeKey);
