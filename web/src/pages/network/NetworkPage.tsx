import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { deviceBreakpoints } from '../../shared/constants';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { useNetworkPageStore } from './hooks/useNetworkPageStore';
import { NetworkControls } from './NetworkControls/NetworkControls';
import { NetworkEditForm } from './NetworkEditForm/NetworkEditForm';
import { NetworkGatewaySetup } from './NetworkGateway/NetworkGateway';
import { NetworkTabs } from './NetworkTabs/NetworkTabs';

export const NetworkPage = () => {
  const {
    network: { getNetworks },
  } = useApi();
  const { LL } = useI18nContext();
  const setPageStore = useNetworkPageStore((state) => state.setState);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  useQuery({
    queryKey: [QueryKeys.FETCH_NETWORKS],
    queryFn: getNetworks,
    onSuccess: (res) => {
      setPageStore({ networks: res });
    },
    refetchOnWindowFocus: false,
  });

  return (
    <PageContainer id="network-page">
      <header>
        <h1>{LL.networkPage.pageTitle()}</h1>
      </header>
      {breakpoint === 'desktop' && <NetworkTabs />}
      <Card className="network-card" hideMobile>
        <NetworkControls />
        <NetworkEditForm />
        <NetworkGatewaySetup />
      </Card>
    </PageContainer>
  );
};
