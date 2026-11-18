import { m } from '../../../../paraglide/messages';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../shared/defguard-ui/components/Badge/types';

export const OnlineBadge = () => {
  return (
    <Badge variant={BadgeVariant.Success} showIcon text={m.devices_connection_online()} />
  );
};
