import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { flatten } from 'lodash-es';
import { useEffect, useMemo } from 'react';

import { isPresent } from '../../../../defguard-ui/utils/isPresent';
import useApi from '../../../../hooks/useApi';
import { useToaster } from '../../../../hooks/useToaster';
import { GatewayStatus } from '../../../../types';
import { GatewaysFloatingStatus } from '../GatewaysFloatingStatus/GatewaysFloatingStatus';
import { GatewaysStatusInfo } from '../GatewaysStatusInfo/GatewaysStatusInfo';

type MappedStats = {
  id: number;
  name: string;
  gateways: GatewayStatus[];
};

export const AllNetworksGatewaysStatus = () => {
  const {
    network: { getAllGatewaysStatus },
  } = useApi();

  const toaster = useToaster();

  const { data, isLoading, isError, error } = useQuery({
    queryKey: ['network', 'gateways'],
    queryFn: getAllGatewaysStatus,
    placeholderData: (perv) => perv,
    refetchInterval: 60 * 1000,
  });

  const [totalConnections, connectedCount] = useMemo(() => {
    if (data) {
      const flat = flatten(Object.values(data));
      const totalCount = flat.length;
      const connectedCount = flat.reduce(
        (ac, current) => ac + (current.connected ? 1 : 0),
        0,
      );
      return [totalCount, connectedCount];
    }
    return [0, 0];
  }, [data]);

  const listData = useMemo(() => {
    if (data) {
      const res: MappedStats[] = [];
      for (const networkId of Object.keys(data)) {
        const gateways = data[networkId];
        if (gateways.length > 0) {
          const name = gateways[0].network_name;
          res.push({
            id: Number(networkId),
            name,
            gateways,
          });
        }
      }
      return res;
    }
    return [];
  }, [data]);

  useEffect(() => {
    if (isPresent(error)) {
      toaster.error('Failed to check full gateways status.');
      console.error(error);
    }
  }, [error, toaster]);

  return (
    <GatewaysStatusInfo
      connectionCount={connectedCount}
      totalCount={totalConnections}
      isError={isError}
      isLoading={isLoading}
    >
      <div className="all-networks-gateways">
        {listData.map((stats) => (
          <div className="network-gateways" key={stats.id}>
            <div className="network">
              <p>{stats.name}</p>
            </div>
            {stats.gateways.map((gateway) => (
              <GatewaysFloatingStatus status={gateway} key={gateway.uid} />
            ))}
          </div>
        ))}
      </div>
    </GatewaysStatusInfo>
  );
};
