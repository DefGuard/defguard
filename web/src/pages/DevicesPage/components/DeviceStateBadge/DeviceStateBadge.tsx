import { m } from '../../../../paraglide/messages';
import { DeviceState, type DeviceStateValue } from '../../../../shared/api/types';
import { Badge } from '../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../shared/defguard-ui/components/Badge/types';
import { BlockedBadge } from '../BlockedBadge/BlockedBadge';

type Props = {
  state: DeviceStateValue;
  removeBackground?: boolean;
};

export const DeviceStateBadge = ({ state, removeBackground = false }: Props) => {
  if (state === DeviceState.Blocked) return <BlockedBadge />;
  return (
    <Badge
      variant={BadgeVariant.Success}
      showIcon
      removeBackground={removeBackground}
      text={m.state_enabled()}
    />
  );
};
