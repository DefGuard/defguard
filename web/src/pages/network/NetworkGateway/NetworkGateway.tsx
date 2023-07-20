import './style.scss';

import { useQuery } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import parse from 'html-react-parser';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../i18n/i18n-react';
import {
  ActionButton,
  ActionButtonVariant,
} from '../../../shared/components/layout/ActionButton/ActionButton';
import { ExpandableCard } from '../../../shared/components/layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../shared/components/layout/MessageBox/MessageBox';
import { GatewaysStatus } from '../../../shared/components/network/GatewaysStatus/GatewaysStatus';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { externalLink } from '../../../shared/links';
import { QueryKeys } from '../../../shared/queries';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkGatewaySetup = () => {
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);
  const toaster = useToaster();
  const { LL } = useI18nContext();
  const {
    network: { getNetworkToken },
  } = useApi();

  const { data: networkToken } = useQuery(
    [QueryKeys.FETCH_NETWORK_TOKEN, selectedNetworkId],
    () => getNetworkToken(selectedNetworkId),
    {
      refetchOnMount: true,
      refetchOnWindowFocus: false,
    },
  );

  const command = useCallback(() => {
    // eslint-disable-next-line max-len
    return `docker run -e DEFGUARD_TOKEN=${networkToken?.token} -e DEFGUARD_GRPC_URL=${networkToken?.grpc_url} --restart unless-stopped --network host --cap-add NET_ADMIN ghcr.io/defguard/gateway:latest`;
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
    [command, toaster, LL.messages],
  );
  return (
    <section className="gateway">
      <header>
        <h2>{LL.gatewaySetup.header()}</h2>
      </header>
      <MessageBox>
        {parse(
          networkToken
            ? LL.gatewaySetup.messages.runCommand({
                setupGatewayDocs: externalLink.gitbook.setup.gateway,
              })
            : LL.gatewaySetup.messages.createNetwork(),
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
        </>
      )}
      <GatewaysStatus networkId={selectedNetworkId} />
    </section>
  );
};
