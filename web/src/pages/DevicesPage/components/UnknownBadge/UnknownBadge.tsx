import { m } from '../../../../paraglide/messages';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../shared/defguard-ui/components/Badge/types';

export const UnknownBadge = () => {
  return <Badge variant={BadgeVariant.Neutral} text={m.devices_unknown()} />;
};
