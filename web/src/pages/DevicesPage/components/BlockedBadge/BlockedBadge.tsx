import { m } from '../../../../paraglide/messages';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../shared/defguard-ui/components/Badge/types';

export const BlockedBadge = () => {
  return (
    <Badge
      variant={BadgeVariant.Critical}
      icon="status-locked"
      showIcon
      text={m.devices_state_blocked()}
    />
  );
};
