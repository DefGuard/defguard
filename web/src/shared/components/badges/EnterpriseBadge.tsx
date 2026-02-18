import { Badge } from '../../defguard-ui/components/Badge/Badge';
import type { BadgeProps } from '../../defguard-ui/components/Badge/types';
import { IconKind } from '../../defguard-ui/components/Icon';

export const enterpriseBadgeProps: BadgeProps = {
  variant: 'warning',
  text: 'Enterprise plan',
  showIcon: true,
  icon: IconKind.StatusPremium,
  iconSize: 16,
};

export const EnterpriseBadge = () => {
  return <Badge {...enterpriseBadgeProps} />;
};
