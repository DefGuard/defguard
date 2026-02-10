import {
  autoUpdate,
  FloatingPortal,
  flip,
  offset,
  shift,
  size,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import clsx from 'clsx';
import { type HTMLProps, useMemo, useState } from 'react';
import type { GatewayStatus } from '../../api/types';
import { Badge } from '../../defguard-ui/components/Badge/Badge';
import type { BadgeVariantValue } from '../../defguard-ui/components/Badge/types';
import { Button } from '../../defguard-ui/components/Button/Button';
import type { IconKindValue } from '../../defguard-ui/components/Icon/icon-types';
import { InteractionBox } from '../../defguard-ui/components/InteractionBox/InteractionBox';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useGatewayWizardStore } from '../../../pages/GatewaySetupPage/useGatewayWizardStore';
import api from '../../api/api';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { Icon } from '../../defguard-ui/components/Icon';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';

type Status = 'all' | 'none' | 'some';

type Props = {
  data: GatewayStatus[];
  showDetails?: boolean;
};

export const GatewaysStatusBadge = ({ data, showDetails = false }: Props) => {
  const enableOpen = showDetails && data.length > 0;
  const [isOpen, setOpen] = useState(false);
  const { refs, floatingStyles, context } = useFloating({
    placement: 'bottom-start',
    open: isOpen,
    onOpenChange: setOpen,
    middleware: [
      flip(),
      shift(),
      offset(8),
      size({
        apply({ rects, elements }) {
          const refWidth = `${rects.reference.width}px`;
          elements.floating.style.minWidth = refWidth;
        },
      }),
    ],
    whileElementsMounted: autoUpdate,
  });

  const click = useClick(context, { toggle: true, enabled: enableOpen });

  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    escapeKey: true,
    outsidePress: true,
  });

  const { getFloatingProps, getReferenceProps } = useInteractions([click, dismiss]);

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

  return (
    <>
      <Badge
        className={clsx('gateways-status-badge', {
          interactive: enableOpen,
        })}
        text={text()}
        icon={icon()}
        variant={variant()}
        showIcon
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        {enableOpen && <Icon icon="arrow-small" rotationDirection="down" size={16} />}
      </Badge>
      {isOpen && (
        <FloatingPortal>
          <FloatingMenu
            ref={refs.setFloating}
            status={data}
            style={{ ...floatingStyles }}
            {...getFloatingProps()}
          />
        </FloatingPortal>
      )}
    </>
  );
};

const FloatingMenu = ({
  status,
  className,
  ...rest
}: { status: GatewayStatus[] } & HTMLProps<HTMLDivElement>) => {
  const networkId = status[0].network_id as number;
  const connected = useMemo(() => status.filter((gw) => gw.connected), [status]);
  const disconnected = useMemo(() => status.filter((gw) => !gw.connected), [status]);
  const navigate = useNavigate();

  const { mutate: removeGw } = useMutation({
    mutationFn: api.location.deleteGateway,
    meta: {
      invalidate: ['network', networkId, 'gateways'],
    },
  });

  return (
    <div className={clsx('gateways-status-floating', className)} {...rest}>
      {connected.length > 0 && (
        <div className="connected">
          <p>Connected</p>
          <ul>
            {connected.map((gw) => (
              <li key={gw.uid}>
                <Badge
                  showIcon
                  removeBackground
                  variant="success"
                  icon="status-attention"
                  text={gw.name ?? gw.hostname}
                />
              </li>
            ))}
          </ul>
        </div>
      )}
      {connected.length > 0 && disconnected.length > 0 && (
        <Divider spacing={ThemeSpacing.Md} />
      )}
      {disconnected.length > 0 && (
        <div className="disconnected">
          <p>Disconnected</p>
          <ul>
            {disconnected.map((gw) => (
              <li key={gw.uid}>
                <Badge
                  removeBackground
                  showIcon
                  icon="status-important"
                  variant="critical"
                  text={gw.name ?? gw.hostname}
                />
                <InteractionBox
                  icon="close"
                  iconSize={20}
                  onClick={() => {
                    removeGw({
                      gatewayId: gw.uid,
                      networkId: networkId,
                    });
                  }}
                />
              </li>
            ))}
          </ul>
        </div>
      )}
      <SizedBox height={ThemeSpacing.Lg} />
      <Button
        iconLeft="network-settings"
        size="big"
        variant="outlined"
        text="Add more gateways"
        onClick={() => {
          useGatewayWizardStore.getState().start({ network_id: networkId });
          navigate({ to: '/setup-gateway', replace: true });
        }}
      />
    </div>
  );
};
