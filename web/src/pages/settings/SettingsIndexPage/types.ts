import z from 'zod';

export const settingsTabsSchema = z.enum([
  'general',
  'notifications',
  'openid',
  'activity',
  'license',
]);

export type SettingsTabValue = z.infer<typeof settingsTabsSchema>;
