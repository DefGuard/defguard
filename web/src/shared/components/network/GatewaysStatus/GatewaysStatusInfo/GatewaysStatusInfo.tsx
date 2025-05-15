import './style.scss';

import clsx from 'clsx';
import { PropsWithChildren, useMemo, useState } from 'react';
import Skeleton from 'react-loading-skeleton';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ArrowSingle } from '../../../../defguard-ui/components/icons/ArrowSingle/ArrowSingle';
import { ArrowSingleDirection } from '../../../../defguard-ui/components/icons/ArrowSingle/types';
import { FloatingMenu } from '../../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import { Label } from '../../../../defguard-ui/components/Layout/Label/Label';
import { GatewayStatusIcon } from '../GatewayStatusIcon';
import { GatewayConnectionStatus } from '../types';

type Props = {
  totalCount: number;
  connectionCount: number;
  isLoading?: boolean;
  isError?: boolean;
  forceStatus?: GatewayConnectionStatus;
} & PropsWithChildren;

export const GatewaysStatusInfo = ({
  children,
  connectionCount,
  totalCount,
  forceStatus,
  isLoading = false,
  isError = false,
}: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.components.gatewaysStatus;
  const [floatingOpen, setOpen] = useState(false);

  const status = useMemo((): GatewayConnectionStatus => {
    if (forceStatus) {
      return forceStatus;
    }
    if (isError) {
      return GatewayConnectionStatus.ERROR;
    }
    if (isLoading) {
      return GatewayConnectionStatus.LOADING;
    }
    if (totalCount === 0 || connectionCount === 0) {
      return GatewayConnectionStatus.DISCONNECTED;
    }
    if (totalCount !== connectionCount) {
      return GatewayConnectionStatus.PARTIAL;
    }
    return GatewayConnectionStatus.CONNECTED;
  }, [connectionCount, forceStatus, isError, isLoading, totalCount]);

  const getInfoText = () => {
    switch (status) {
      case GatewayConnectionStatus.LOADING:
        return '';
      case GatewayConnectionStatus.ERROR:
        return localLL.states.error();
      case GatewayConnectionStatus.DISCONNECTED:
        return localLL.states.none();
      case GatewayConnectionStatus.PARTIAL:
        return localLL.states.some({
          count: connectionCount,
        });
      case GatewayConnectionStatus.CONNECTED:
        return localLL.states.all({
          count: connectionCount,
        });
    }
  };

  return (
    <div className="gateways-status-info">
      <Label>{localLL.label()}</Label>
      <FloatingMenuProvider onOpenChange={setOpen} open={floatingOpen} placement="bottom">
        <FloatingMenuTrigger asChild>
          <div
            className="info-track"
            onClick={() => {
              if (totalCount > 0) {
                setOpen(true);
              }
            }}
          >
            {isLoading && <Skeleton />}
            {!isLoading && (
              <div
                className={clsx('info', {
                  disconnected:
                    isError || status === GatewayConnectionStatus.DISCONNECTED,
                  connected: status === GatewayConnectionStatus.CONNECTED,
                  partial: status === GatewayConnectionStatus.PARTIAL,
                })}
              >
                <p>{getInfoText()}</p>
                <GatewayStatusIcon status={status} />
              </div>
            )}
            {totalCount > 0 && <ArrowSingle direction={ArrowSingleDirection.DOWN} />}
          </div>
        </FloatingMenuTrigger>
        <FloatingMenu className={clsx('gateways-status-floating-menu')}>
          {children}
        </FloatingMenu>
      </FloatingMenuProvider>
    </div>
  );
};
