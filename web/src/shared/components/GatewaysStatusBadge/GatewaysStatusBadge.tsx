import { useMemo } from 'react';
import type { GatewayStatus } from '../../api/types';
import { Badge } from '../../defguard-ui/components/Badge/Badge';
import type { BadgeVariantValue } from '../../defguard-ui/components/Badge/types';
import type { IconKindValue } from '../../defguard-ui/components/Icon/icon-types';

type Status = 'all' | 'none' | 'some';

type Props = {
  data: GatewayStatus[];
};

export const GatewaysStatusBadge = ({ data }: Props) => {
  const connectedLength = useMemo(() => data.filter((gw) => gw.connected).length, [data]);

  const status = useMemo((): Status => {
    if (connectedLength === 0 || data.length === 0) {
      return 'none';
    }
    if (connectedLength === data.length) {
      return 'all';
    }
    return 'some';
  }, [data.length, connectedLength]);

  const text = () => {
    switch (status) {
      case 'all':
        return 'Gateway (all) connected';
      case 'some':
        return `Gateway (${connectedLength}) connected`;
      case 'none':
        return 'None connected';
    }
  };

  const icon = (): IconKindValue => {
    switch (status) {
      case 'all':
        return 'status-available';
      case 'some':
        return 'status-attention';
      case 'none':
        return 'status-important';
    }
  };

  const variant = (): BadgeVariantValue => {
    switch (status) {
      case 'all':
        return 'success';
      case 'none':
        return 'critical';
      case 'some':
        return 'warning';
    }
  };

  return <Badge text={text()} icon={icon()} variant={variant()} showIcon />;
};
