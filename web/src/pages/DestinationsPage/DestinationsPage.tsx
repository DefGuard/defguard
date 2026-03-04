import { useQuery } from '@tanstack/react-query';
import { Suspense, useMemo, useState } from 'react';
import { AclDeploymentState, type AclDeploymentStateValue } from '../../shared/api/types';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getDestinationsCountQueryOptions } from '../../shared/query';
import { DestinationDeployedTab } from './tabs/DestinationDeployedTab/DestinationDeployedTab';
import { DestinationPendingTab } from './tabs/DestinationPendingTab/DestinationPendingTab';

export const DestinationsPage = () => {
  const { data: destinationsCount } = useQuery(getDestinationsCountQueryOptions);

  const [activeTab, setActiveTab] = useState<AclDeploymentStateValue>(
    AclDeploymentState.Applied,
  );

  const pendingCount = destinationsCount?.pending ?? 0;
  const pendingTitle = pendingCount ? `Pending (${pendingCount})` : 'Pending';
  const pendingIcon = pendingCount > 0 ? IconKind.AttentionFilled : undefined;

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
        title: pendingTitle,
        icon: pendingIcon,
      },
    ],
    [activeTab, pendingIcon, pendingTitle],
  );

  return (
    <Page title="Destinations" id="destination-page">
      <TablePageLayout>
        <Tabs items={tabs} />
        <Suspense fallback={<TableSkeleton />}>
          {activeTab === AclDeploymentState.Applied && <DestinationDeployedTab />}
          {activeTab === AclDeploymentState.Modified && <DestinationPendingTab />}
        </Suspense>
      </TablePageLayout>
    </Page>
  );
};
