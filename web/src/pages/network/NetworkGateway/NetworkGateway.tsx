import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';
import ReactMarkdown from 'react-markdown';

import { useI18nContext } from '../../../i18n/i18n-react';
import { GatewaysStatus } from '../../../shared/components/network/GatewaysStatus/GatewaysStatus';
import { ActionButton } from '../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import useApi from '../../../shared/hooks/useApi';
import { useClipboard } from '../../../shared/hooks/useClipboard';
import { externalLink } from '../../../shared/links';
import { QueryKeys } from '../../../shared/queries';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';
import { AddComponentBox } from '../../users/shared/components/AddComponentBox/AddComponentBox';
import { useAddGatewayModal } from './modals/hooks/useAddGatewayModal';
import { AddGatewayModal } from './modals/AddGatewayModal';
import { GatewayCard } from './components/GatewayCard';
import { EditGatewayModal } from './modals/EditGatewayModal';

export const NetworkGatewaySetup = () => {
  const { writeToClipboard } = useClipboard();
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);
  const { LL } = useI18nContext();
  const {
    network: {
      getNetworkToken,
      gateway: { getAllGateways },
    },
  } = useApi();

  const { data: networkToken } = useQuery(
    [QueryKeys.FETCH_NETWORK_TOKEN, selectedNetworkId],
    () => getNetworkToken(selectedNetworkId),
    {
      refetchOnMount: true,
      refetchOnWindowFocus: false,
    },
  );

  const { data: gateways } = useQuery(
    [QueryKeys.FETCH_ALL_GATEWAYS, selectedNetworkId],
    () => getAllGateways(selectedNetworkId),
    {
      refetchOnMount: true,
      refetchOnWindowFocus: false,
    },
  );

  const command = useCallback(() => {
    // eslint-disable-next-line max-len
    return `docker run -e DEFGUARD_TOKEN=${networkToken?.token} --restart unless-stopped --network host --cap-add NET_ADMIN ghcr.io/defguard/gateway:latest`;
  }, [networkToken]);

  const returnNetworkToken = useCallback(() => {
    return `${networkToken?.token}`;
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

  const getNetworkTokenActions = useMemo(
    () => [
      <ActionButton
        key={1}
        variant={ActionButtonVariant.COPY}
        onClick={() => {
          writeToClipboard(returnNetworkToken());
        }}
      />,
    ],
    [returnNetworkToken, writeToClipboard],
  );

  // TODO: consider a better way to redirect to the gateway releases page
  const handleSubmit = () => {
    window.location.href = 'https://github.com/DefGuard/gateway/releases';
  };

  const openAddGatewayModal = useAddGatewayModal((state) => state.open);

  return (
    <section className="gateway">
      <AddGatewayModal />
      {/* <EditGatewayModal /> */}
      <section className="header-section">
        <h2>Gateways</h2>
      </section>
      <section className="gateway-list">
        {gateways?.map((gateway) => <GatewayCard key={gateway.id} gateway={gateway} />)}
        <AddComponentBox
          data-testid="add-gateway"
          text={'Add gateway'}
          // disabled={!appInfo?.network_present || !canManageDevices}
          callback={openAddGatewayModal}
        />
      </section>
      <section className="header-section">
        <h2>{LL.gatewaySetup.header.main()}</h2>
        {/* {parse(
          LL.gatewaySetup.messages.runCommand({
            setupGatewayDocs: externalLink.gitbook.setup.gateway,
          }),
        )} */}
        <ReactMarkdown>
          {LL.gatewaySetup.messages.runCommand({
            setupGatewayDocs: externalLink.gitbook.setup.gateway,
          })}
        </ReactMarkdown>
      </section>
      <MessageBox>
        <ReactMarkdown>
          {networkToken
            ? LL.gatewaySetup.messages.authToken({
              setupGatewayDocs: externalLink.gitbook.setup.gateway,
            })
            : LL.gatewaySetup.messages.createNetwork()}
        </ReactMarkdown>
      </MessageBox>
      {networkToken && (
        <>
          <ExpandableCard
            title={LL.gatewaySetup.card.authToken()}
            disableExpand={true}
            expanded={true}
            actions={getNetworkTokenActions}
          >
            <p>{returnNetworkToken()}</p>
          </ExpandableCard>
        </>
      )}
      <h3>{LL.gatewaySetup.header.dockerBasedGatewaySetup()}</h3>
      <MessageBox>
        <ReactMarkdown>
          {networkToken
            ? LL.gatewaySetup.messages.dockerBasedGatewaySetup({
              setupGatewayDocs: externalLink.gitbook.setup.gateway,
            })
            : LL.gatewaySetup.messages.createNetwork()}
        </ReactMarkdown>
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
      <h3>{LL.gatewaySetup.header.fromPackage()}</h3>
      <MessageBox>
        <ReactMarkdown>
          {LL.gatewaySetup.messages.fromPackage({
            setupGatewayDocs: externalLink.gitbook.setup.gateway,
          })}
        </ReactMarkdown>
      </MessageBox>
      <Button
        size={ButtonSize.LARGE}
        styleVariant={ButtonStyleVariant.PRIMARY}
        text="Available Packages"
        onClick={() => handleSubmit()}
      />
      <h3>{LL.gatewaySetup.header.oneLineInstall()}</h3>
      <MessageBox>
        <ReactMarkdown>{LL.gatewaySetup.messages.oneLineInstall()}</ReactMarkdown>
      </MessageBox>
      {/* <GatewaysStatus networkId={selectedNetworkId} /> */}
    </section>
  );
};
