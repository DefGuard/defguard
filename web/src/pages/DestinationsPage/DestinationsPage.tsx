import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import api from '../../shared/api/api';
import { AclDeploymentState, type AclDeploymentStateValue } from '../../shared/api/types';
import { Page } from '../../shared/components/Page/Page';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { DestinationDeployedTab } from './tabs/DestinationDeployedTab/DestinationDeployedTab';
import { DestinationPendingTab } from './tabs/DestinationPendingTab/DestinationPendingTab';

export const DestinationsPage = () => {
  const [activeTab, setActiveTab] = useState<AclDeploymentStateValue>(
    AclDeploymentState.Applied,
  );

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        active: activeTab === AclDeploymentState.Applied,
        onClick: () => {
          setActiveTab(AclDeploymentState.Applied);
        },
        title: 'Deployed',
      },
      {
        active: activeTab === AclDeploymentState.Modified,
        onClick: () => {
          setActiveTab(AclDeploymentState.Modified);
        },
        title: 'Pending',
      },
    ],
    [activeTab],
  );

  const { data: destinationsData } = useQuery({
    queryFn: api.acl.destination.getDestinations,
    queryKey: ['acl', 'destination'],
    select: (resp) => resp.data,
  });

  const applied = useMemo(
    () => destinationsData?.filter((item) => item.state === AclDeploymentState.Applied),
    [destinationsData],
  );

  const pending = useMemo(
    () => destinationsData?.filter((item) => item.state === AclDeploymentState.Modified),
    [destinationsData],
  );

  return (
    <Page title="Destinations" id="destination-page">
      <TablePageLayout>
        <Tabs items={tabs} />
        {isPresent(pending) && isPresent(applied) && (
          <>
            {activeTab === AclDeploymentState.Applied && (
              <DestinationDeployedTab destinations={applied} />
            )}
            {activeTab === AclDeploymentState.Modified && (
              <DestinationPendingTab destinations={pending} />
            )}
          </>
        )}
      </TablePageLayout>
    </Page>
  );
};
