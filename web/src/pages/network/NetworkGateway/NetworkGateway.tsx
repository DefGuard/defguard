import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import parse from 'html-react-parser';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../i18n/i18n-react';
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
  const { LL } = useI18nContext();
  const {
    network: { getGatewayStatus, getNetworkToken },
  } = useApi();
  const queryClient = useQueryClient();
  const { data: gatewayStatus, isLoading: statusLoading } = useQuery(
    [QueryKeys.FETCH_GATEWAY_STATUS],
    getGatewayStatus,
    {
      onError: (err) => {
        toaster.error(LL.gatewaySetup.messages.statusError());
        console.error(err);
      },
      refetchOnWindowFocus: false,
    }
  );

  const { data: networkToken } = useQuery([QueryKeys.FETCH_NETWORK_TOKEN], () =>
    getNetworkToken('1')
  );

  const command = useCallback(() => {
    // eslint-disable-next-line max-len
    return `docker run -e DEFGUARD_TOKEN=${networkToken?.token} -e DEFGUARD_GRPC_URL=http://localhost:50055 --restart unless-stopped --network host --cap-add NET_ADMIN ghcr.io/defguard/gateway:latest`;
  }, [networkToken]);

  const getActions = useMemo(
    () => [
      <ActionButton
        key={1}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          clipboard
            .write(command())
            .then(() => {
              toaster.success(LL.messages.successClipboard());
            })
            .catch((err) => {
              toaster.error(LL.messages.clipboardError());
              console.error(err);
            });
        }}
      />,
    ],
    [command, toaster, LL.messages]
  );
  return (
    <section className="gateway">
      <header>
        <h2>{LL.gatewaySetup.header()}</h2>
      </header>
      <Card>
        <MessageBox>
          {parse(
            networkToken
              ? LL.gatewaySetup.messages.runCommand()
              : LL.gatewaySetup.messages.createNetwork()
          )}
        </MessageBox>
        {networkToken && (
          <>
            <ExpandableCard
              title={LL.gatewaySetup.card.title()}
              disableExpand={true}
              expanded={true}
              actions={getActions}
            >
              <p>{command()}</p>
            </ExpandableCard>
            <div className="status">
              <Button
                size={ButtonSize.BIG}
                styleVariant={ButtonStyleVariant.PRIMARY}
                text={LL.gatewaySetup.controls.status()}
                loading={statusLoading}
                onClick={() => {
                  if (!statusLoading) {
                    queryClient.invalidateQueries([QueryKeys.FETCH_GATEWAY_STATUS]);
                  }
                }}
              />
              {!gatewayStatus?.connected && !statusLoading && (
                <MessageBox type={MessageBoxType.ERROR}>
                  {parse(LL.gatewaySetup.messages.noConnection())}
                </MessageBox>
              )}
              {gatewayStatus?.connected && !statusLoading && (
                <MessageBox type={MessageBoxType.SUCCESS}>
                  {parse(LL.gatewaySetup.messages.connected())}
                </MessageBox>
              )}
            </div>
          </>
        )}
      </Card>
    </section>
  );
};
