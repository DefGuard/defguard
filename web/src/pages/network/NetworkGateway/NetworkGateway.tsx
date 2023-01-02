import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import { useMemo } from 'react';

import {
  ActionButton,
  ActionButtonVariant,
} from '../../../shared/components/layout/ActionButton/ActionButton';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { Card } from '../../../shared/components/layout/Card/Card';
import { ExpandableCard } from '../../../shared/components/layout/ExpandableCard/ExpandableCard';
import MessageBox, {
  MessageBoxType,
} from '../../../shared/components/layout/MessageBox/MessageBox';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../shared/queries';

export const NetworkGatewaySetup = () => {
  const toaster = useToaster();
  const {
    network: { getGatewayStatus, getNetworkToken },
  } = useApi();
  const queryClient = useQueryClient();
  const { data: gatewayStatus, isLoading: statusLoading } = useQuery(
    [QueryKeys.FETCH_GATEWAY_STATUS],
    getGatewayStatus,
    {
      onError: (err) => {
        toaster.error('Failed to get gateway status');
        console.error(err);
      },
      refetchOnWindowFocus: false,
    }
  );

  const { data: networkToken } = useQuery([QueryKeys.FETCH_NETWORK_TOKEN], () =>
    getNetworkToken('1')
  );

  const command = `docker run -e DEFGUARD_TOKEN=${networkToken?.token} registry.teonite.net/defguard/wireguard:latest`;

  const getActions = useMemo(
    () => [
      <ActionButton
        key={1}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          clipboard
            .write(command)
            .then(() => {
              toaster.success('Command copied.');
            })
            .catch((err) => {
              toaster.error('Clipboard is not accessible.');
              console.error(err);
            });
        }}
      />,
    ],
    [command, toaster]
  );
  return (
    <section className="gateway">
      <header>
        <h2>Gateway server setup</h2>
      </header>
      <Card>
        <MessageBox>
          <p>
            Please use command below on your gateway server. If you don{"'"}t
            know how, or have some issues please visit our{' '}
            <a>detailed documentation page</a>.
          </p>
        </MessageBox>
        <ExpandableCard
          title="Gateway setup command"
          disableExpand={true}
          expanded={true}
          actions={getActions}
        >
          <p>{command}</p>
        </ExpandableCard>
        <div className="status">
          <Button
            size={ButtonSize.BIG}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text="Check connection status"
            loading={statusLoading}
            onClick={() => {
              if (!statusLoading) {
                queryClient.invalidateQueries([QueryKeys.FETCH_GATEWAY_STATUS]);
              }
            }}
          />
          {!gatewayStatus?.connected && !statusLoading && (
            <MessageBox type={MessageBoxType.ERROR}>
              <p>No connection established, please run provided command.</p>
            </MessageBox>
          )}
          {gatewayStatus?.connected && !statusLoading && (
            <MessageBox type={MessageBoxType.SUCCESS}>
              <p>Gateway connected.</p>
            </MessageBox>
          )}
        </div>
      </Card>
    </section>
  );
};
