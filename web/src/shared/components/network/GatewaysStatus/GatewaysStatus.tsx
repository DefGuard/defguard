import './style.scss';

import { useQuery } from '@tanstack/react-query';
import classNames from 'classnames';
import { motion, TargetAndTransition } from 'framer-motion';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ColorsRGB } from '../../../constants';
import useApi from '../../../hooks/useApi';
import { useToaster } from '../../../hooks/useToaster';
import { QueryKeys } from '../../../queries';
import { Label } from '../../layout/Label/Label';
import LoaderSpinner from '../../layout/LoaderSpinner/LoaderSpinner';
import SvgIconArrowSingle from '../../svg/IconArrowSingle';
import { GatewayStatusIcon } from './GatewayStatusIcon';

type Props = {
  networkId: number;
};

const REFETCH_INTERVAL = 5 * 1000;

export const GatewaysStatus = ({ networkId }: Props) => {
  const toaster = useToaster();
  const {
    network: { getNetwork },
  } = useApi();
  const { LL } = useI18nContext();
  const {
    data,
    isError,
    isLoading: queryLoading,
  } = useQuery(
    [`${QueryKeys.FETCH_NETWORK}_${networkId}`, networkId],
    () => getNetwork(networkId),
    {
      refetchInterval: REFETCH_INTERVAL,
      onError: () => {
        toaster.error(LL.components.gatewaysStatus.messages.error());
      },
    }
  );

  const isLoading = (queryLoading && !data) || !data;

  const getStatus = useMemo(() => {
    if (isLoading) {
      return GatewayConnectionStatus.LOADING;
    }
    if (isError) {
      return GatewayConnectionStatus.ERROR;
    }
    if (data) {
      const connected = data.gateways?.map((g) => g.connected) ?? [];
      if (connected.length === 0) {
        return GatewayConnectionStatus.DISCONNECTED;
      }
      if (connected.length === data.gateways?.length) {
        return GatewayConnectionStatus.CONNECTED;
      }
      return GatewayConnectionStatus.PARTIAL;
    }
    return GatewayConnectionStatus.ERROR;
  }, [data, isError, isLoading]);

  const getMessage = useMemo((): string => {
    switch (getStatus) {
      case GatewayConnectionStatus.ERROR:
        return LL.components.gatewaysStatus.states.error();
      case GatewayConnectionStatus.DISCONNECTED:
        return LL.components.gatewaysStatus.states.disconnected();
      case GatewayConnectionStatus.PARTIAL:
        return LL.components.gatewaysStatus.states.partial();
      case GatewayConnectionStatus.CONNECTED:
        return LL.components.gatewaysStatus.states.connected();
      case GatewayConnectionStatus.LOADING:
        return LL.components.gatewaysStatus.states.loading();
      default:
        return LL.components.gatewaysStatus.states.error();
    }
  }, [LL.components.gatewaysStatus.states, getStatus]);

  const getAnimate = useMemo(() => {
    const res: TargetAndTransition = {
      color: ColorsRGB.Error,
    };
    switch (getStatus) {
      case GatewayConnectionStatus.CONNECTED:
        res.color = ColorsRGB.Success;
        break;
      case GatewayConnectionStatus.ERROR:
        res.color = ColorsRGB.Error;
        break;
      case GatewayConnectionStatus.PARTIAL:
        res.color = ColorsRGB.Warning;
        break;
      case GatewayConnectionStatus.DISCONNECTED:
        res.color = ColorsRGB.Error;
        break;
      case GatewayConnectionStatus.LOADING:
        res.color = ColorsRGB.GrayLight;
        break;
    }
    return res;
  }, [getStatus]);

  const cn = useMemo(
    () =>
      classNames(
        'network-gateways-connection',
        `status-${getStatus.valueOf().toLowerCase()}`
      ),
    [getStatus]
  );

  return (
    <div className={cn}>
      <Label>{LL.components.gatewaysStatus.label()}</Label>
      <div className="status-container">
        <div className="status">
          <motion.span animate={getAnimate} initial={false}>
            {getMessage}
          </motion.span>
          {!isLoading && <GatewayStatusIcon status={getStatus} />}
        </div>
        {isLoading ? <LoaderSpinner size={12} /> : <SvgIconArrowSingle />}
      </div>
    </div>
  );
};

export enum GatewayConnectionStatus {
  CONNECTED = 'CONNECTED',
  PARTIAL = 'PARTIAL',
  DISCONNECTED = 'DISCONNECTED',
  ERROR = 'ERROR',
  LOADING = 'LOADING',
}
