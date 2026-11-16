import { m } from '../../../../paraglide/messages';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../shared/defguard-ui/components/Badge/types';

type Props = {
  connected: boolean;
  removeBackground?: boolean;
};

export const ConnectionBadge = ({ connected, removeBackground = false }: Props) => {
  return (
    <Badge
      variant={connected ? BadgeVariant.Success : BadgeVariant.Neutral}
      showIcon
      removeBackground={removeBackground}
      text={connected ? m.devices_connection_online() : m.devices_connection_offline()}
    />
  );
};
