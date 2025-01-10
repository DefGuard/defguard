import './style.scss';

import { autoUpdate, offset, useFloating } from '@floating-ui/react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import classNames from 'classnames';
import { AnimatePresence, motion, TargetAndTransition } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import ClickAwayListener from 'react-click-away-listener';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ColorsRGB } from '../../../constants';
import { Label } from '../../../defguard-ui/components/Layout/Label/Label';
import { LoaderSpinner } from '../../../defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import useApi from '../../../hooks/useApi';
import { useToaster } from '../../../hooks/useToaster';
import { QueryKeys } from '../../../queries';
import { GatewayStatus } from '../../../types';
import SvgIconArrowSingle from '../../svg/IconArrowSingle';
import IconInfoError from '../../svg/IconInfoError';
import SvgIconInfoSuccess from '../../svg/IconInfoSuccess';
import SvgIconX from '../../svg/IconX';
import { GatewayStatusIcon } from './GatewayStatusIcon';
import { GatewayConnectionStatus } from './types';

type Props = {
  networkId: number;
};

const REFETCH_INTERVAL = 5 * 1000;

export const GatewaysStatus = ({ networkId }: Props) => {
  const toaster = useToaster();
  const {
    network: { getGatewaysStatus, deleteGateway },
  } = useApi();
  const { LL } = useI18nContext();
  const queryClient = useQueryClient();
  const [floatingOpen, setFloatingOpen] = useState(false);
  const { x, y, strategy, refs } = useFloating({
    placement: 'bottom',
    strategy: 'fixed',
    open: floatingOpen,
    onOpenChange: setFloatingOpen,
    whileElementsMounted: (refElement, floatingElement, updateFunc) =>
      autoUpdate(refElement, floatingElement, updateFunc),
    middleware: [offset(5)],
  });

  const {
    data,
    isError,
    error: fetchError,
    isLoading: queryLoading,
  } = useQuery({
    queryFn: () => getGatewaysStatus(networkId),
    queryKey: [QueryKeys.FETCH_NETWORK_GATEWAYS_STATUS, networkId],
    refetchInterval: REFETCH_INTERVAL,
    enabled: !isUndefined(networkId),
  });

  useEffect(() => {
    if (fetchError) {
      toaster.error(LL.components.gatewaysStatus.messages.error());
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fetchError]);

  const { mutate: deleteGatewayMutation } = useMutation({
    mutationFn: deleteGateway,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_NETWORK_GATEWAYS_STATUS],
      });
    },
    onError: (err) => {
      toaster.error(LL.components.gatewaysStatus.messages.deleteError());
      console.error(err);
    },
  });

  const isLoading = (queryLoading && !data) || !data;

  const getStatus = useMemo(() => {
    if (isLoading) {
      return GatewayConnectionStatus.LOADING;
    }
    if (isError) {
      return GatewayConnectionStatus.ERROR;
    }
    if (data) {
      const connected = data.filter((g) => g.connected) ?? [];
      if (connected.length === 0) {
        return GatewayConnectionStatus.DISCONNECTED;
      }
      if (connected.length === data.length) {
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
        `status-${getStatus.valueOf().toLowerCase()}`,
      ),
    [getStatus],
  );

  return (
    <>
      <div className={cn}>
        <Label>{LL.components.gatewaysStatus.label()}</Label>
        <div
          className="status-container"
          ref={refs.setReference}
          onClick={() => setFloatingOpen((state) => !state)}
        >
          <div className="status">
            <motion.span animate={getAnimate} initial={false}>
              {getMessage}
            </motion.span>
            {!isLoading && <GatewayStatusIcon status={getStatus} />}
          </div>
          {isLoading ? <LoaderSpinner size={12} /> : <SvgIconArrowSingle />}
        </div>
      </div>
      <AnimatePresence mode="wait">
        {floatingOpen && data && data?.length > 0 && (
          <ClickAwayListener onClickAway={() => setFloatingOpen(false)}>
            <motion.div
              className="floating-ui-gateways-status"
              ref={refs.setFloating}
              style={{
                position: strategy,
                top: y ?? 0,
                left: x ?? 0,
              }}
              initial={{
                opacity: 0,
              }}
              animate={{
                opacity: 1,
              }}
              exit={{
                opacity: 0,
              }}
              transition={{
                duration: 0.2,
              }}
            >
              {data?.map((g) => (
                <GatewayStatusRow
                  key={g.hostname}
                  status={g}
                  onDismiss={() =>
                    deleteGatewayMutation({
                      networkId,
                      gatewayId: g.uid,
                    })
                  }
                />
              ))}
            </motion.div>
          </ClickAwayListener>
        )}
      </AnimatePresence>
    </>
  );
};

type GatewayStatusRowProps = {
  status: GatewayStatus;
  onDismiss: () => void;
};

const GatewayStatusRow = ({ status, onDismiss }: GatewayStatusRowProps) => {
  const [loading, setLoading] = useState(false);
  const cn = () =>
    classNames('gateway-status-row', {
      disconnected: !status.connected,
    });

  return (
    <div className={cn()}>
      <div className="icon-container">
        {status.connected ? <SvgIconInfoSuccess /> : <IconInfoError />}
      </div>
      <div className="info-container">
        <p className="location">{status.name}</p>
        <p className="hostname">{status.hostname}</p>
      </div>
      {!status.connected && (
        <button
          className="gateway-dismiss"
          onClick={() => {
            setLoading(true);
            onDismiss();
          }}
          disabled={loading}
        >
          {!loading ? <SvgIconX /> : <LoaderSpinner size={16} />}
        </button>
      )}
    </div>
  );
};
