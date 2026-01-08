import z from 'zod';

export const themeSchema = z.enum(['light', 'dark']);

export type ThemeKey = z.infer<typeof themeSchema>;
