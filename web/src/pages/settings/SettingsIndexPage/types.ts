import z from 'zod';
import { m } from '../../../paraglide/messages';
import type { BadgeProps } from '../../../shared/defguard-ui/components/Badge/types';

export const settingsTabsSchema = z.enum([
  'general',
  'notifications',
  'identity',
  'activity',
  'certs',
  'license',
]);

export type SettingsTabValue = z.infer<typeof settingsTabsSchema>;

export const getConfiguredBadge = (): BadgeProps => ({
  text: m.state_configured(),
  icon: 'status-available',
  iconSize: 16,
  variant: 'success',
  showIcon: true,
});

export const getNotConfiguredBadge = (): BadgeProps => ({
  text: m.state_not_configured(),
  variant: 'critical',
});
