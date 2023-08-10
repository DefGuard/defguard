import './style.scss';

import { useQuery } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../i18n/i18n-react';
import { GatewaysStatus } from '../../../shared/components/network/GatewaysStatus/GatewaysStatus';
import { ActionButton } from '../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { ExpandableCard } from '../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import useApi from '../../../shared/hooks/useApi';
import { useClipboard } from '../../../shared/hooks/useClipboard';
import { externalLink } from '../../../shared/links';
import { QueryKeys } from '../../../shared/queries';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkGatewaySetup = () => {
  const { writeToClipboard } = useClipboard();
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);
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
          writeToClipboard(command());
        }}
      />,
    ],
    [command, writeToClipboard],
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
