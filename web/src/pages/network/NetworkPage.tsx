import './style.scss';

import { useQuery } from '@tanstack/react-query';

import { useI18nContext } from '../../i18n/i18n-react';
import { Card } from '../../shared/components/layout/Card/Card';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
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
      <NetworkTabs />
      <Card className="network-card">
        <NetworkControls />
        <NetworkEditForm />
        <NetworkGatewaySetup />
      </Card>
    </PageContainer>
  );
};
