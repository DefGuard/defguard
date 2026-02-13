import { Badge } from '../../defguard-ui/components/Badge/Badge';
import type { BadgeProps } from '../../defguard-ui/components/Badge/types';
import { IconKind } from '../../defguard-ui/components/Icon';

export const businessBadgeProps: BadgeProps = {
  variant: 'plan',
  showIcon: true,
  icon: IconKind.StatusPremium,
  iconSize: 16,
  text: 'Business plan',
};

export const BusinessBadge = () => {
  return <Badge {...businessBadgeProps} />;
};
