import z from 'zod';
import type { BadgeProps } from '../../../shared/defguard-ui/components/Badge/types';

export const settingsTabsSchema = z.enum([
  'general',
  'notifications',
  'identity',
  'activity',
  'license',
]);

export type SettingsTabValue = z.infer<typeof settingsTabsSchema>;

export const configuredBadge: BadgeProps = {
  text: 'Configured',
  icon: 'status-available',
  iconSize: 16,
  variant: 'success',
  showIcon: true,
};

export const notConfiguredBadge: BadgeProps = {
  text: 'Not configured',
  variant: 'critical',
};
