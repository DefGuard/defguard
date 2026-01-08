import type { BadgeProps } from '../../defguard-ui/components/Badge/types';
import type { IconKindValue } from '../../defguard-ui/components/Icon';

export interface EditHeaderProps {
  title: string;
  icon?: IconKindValue;
  badgeProps?: BadgeProps;
  subtitle?: string;
}
