import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';

import useApi from '../../../../hooks/useApi';
import { GatewaysFloatingStatus } from '../GatewaysFloatingStatus/GatewaysFloatingStatus';
import { GatewaysStatusInfo } from '../GatewaysStatusInfo/GatewaysStatusInfo';

type Props = {
  networkId: number;
};
export const NetworkGatewaysStatus = ({ networkId }: Props) => {
  const {
    network: { getGatewaysStatus },
  } = useApi();

  const { data, isLoading, isError } = useQuery({
    queryKey: ['network', networkId, 'gateways'],
    queryFn: () => getGatewaysStatus(networkId),
  });

  const [totalConnections, connectedCount] = useMemo(() => {
    if (!data) return [0, 0];
    const total = data.length;
    const connected = data.reduce(
      (count, status) => count + (status.connected ? 1 : 0),
      0,
    );
    return [total, connected];
  }, [data]);

  return (
    <GatewaysStatusInfo
      connectionCount={connectedCount}
      totalCount={totalConnections}
      isError={isError}
      isLoading={isLoading}
    >
      {data?.map((status) => (
        <GatewaysFloatingStatus status={status} key={status.uid} />
      ))}
    </GatewaysStatusInfo>
  );
};
