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

  const returnNetworkToken = useCallback(() => {
    return `${networkToken}`;
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
      <h2>{LL.gatewaySetup.header.main()}</h2>
      {/* {parse(
        networkToken
          ? LL.gatewaySetup.messages.runCommand({
              setupGatewayDocs: externalLink.gitbook.setup.gateway,
            })
          : LL.gatewaySetup.messages.createNetwork(),
      )} */}
      {parse(
        LL.gatewaySetup.messages.runCommand({
          setupGatewayDocs: externalLink.gitbook.setup.gateway,
        }),
      )}
      {/* Authentication Token */}
      <MessageBox>
        {parse(
          networkToken
            ? LL.gatewaySetup.messages.authToken({
                setupGatewayDocs: externalLink.gitbook.setup.gateway,
              })
            : LL.gatewaySetup.messages.createNetwork(),
        )}
      </MessageBox>
      {networkToken && (
        <>
          <ExpandableCard
            title={LL.gatewaySetup.card.authToken()}
            disableExpand={true}
            expanded={true}
            actions={getActions}
          >
            <p>{returnNetworkToken()}</p>
          </ExpandableCard>
        </>
      )}
      {/* Docker Based Gateway Setup */}
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
      {/* From Package */}
      <h3>{LL.gatewaySetup.header.fromPackage()}</h3>
      <MessageBox>
        {parse(
          LL.gatewaySetup.messages.fromPackage({
            setupGatewayDocs: externalLink.gitbook.setup.gateway,
          }),
        )}
      </MessageBox>
      {/* One Line Install */}
      <h3>{LL.gatewaySetup.header.oneLineInstall()}</h3>
      <MessageBox>{parse(LL.gatewaySetup.messages.oneLineInstall())}</MessageBox>
      {/* Gateway Status */}
      <GatewaysStatus networkId={selectedNetworkId} />
    </section>
  );
};
