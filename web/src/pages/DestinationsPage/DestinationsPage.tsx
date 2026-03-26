import { useQuery } from '@tanstack/react-query';
import { useNavigate, useSearch } from '@tanstack/react-router';
import { Suspense, useCallback, useEffect, useMemo } from 'react';
import {
  AclListTab,
  type AclListTabValue,
  getCanonicalAclListUrlSearch,
} from '../../shared/aclTabs';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getDestinationsCountQueryOptions } from '../../shared/query';
import { DeleteAliasDestinationConfirmModal } from '../Acl/components/DeleteAliasDestinationConfirmModal/DeleteAliasDestinationConfirmModal';
import { DestinationDeployedTab } from './tabs/DestinationDeployedTab/DestinationDeployedTab';
import { DestinationPendingTab } from './tabs/DestinationPendingTab/DestinationPendingTab';

export const DestinationsPage = () => {
  const navigate = useNavigate({ from: '/acl/destinations' });
  const search = useSearch({ from: '/_authorized/_default/acl/destinations' });
  const activeTab = search.tab;

  useEffect(() => {
    if (window.location.search === getCanonicalAclListUrlSearch(activeTab)) {
      return;
    }

    void navigate({ search: { tab: activeTab }, replace: true });
  }, [activeTab, navigate]);

  const { data: destinationsCount } = useQuery(getDestinationsCountQueryOptions);

  const setActiveTab = useCallback(
    (tab: AclListTabValue) => {
      navigate({ search: { tab } });
    },
    [navigate],
  );

  const pendingCount = destinationsCount?.pending ?? 0;
  const pendingTitle = pendingCount ? `Pending (${pendingCount})` : 'Pending';
  const pendingIcon = pendingCount > 0 ? IconKind.AttentionFilled : undefined;

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        active: activeTab === AclListTab.Deployed,
        onClick: () => setActiveTab(AclListTab.Deployed),
        title: 'Deployed',
      },
      {
        active: activeTab === AclListTab.Pending,
        onClick: () => setActiveTab(AclListTab.Pending),
        title: pendingTitle,
        icon: pendingIcon,
      },
    ],
    [activeTab, pendingIcon, pendingTitle, setActiveTab],
  );

  return (
    <Page title="Destinations" id="destination-page">
      <TablePageLayout>
        <Tabs items={tabs} />
        <Suspense fallback={<TableSkeleton />}>
          {activeTab === AclListTab.Deployed && <DestinationDeployedTab />}
          {activeTab === AclListTab.Pending && <DestinationPendingTab />}
        </Suspense>
        <DeleteAliasDestinationConfirmModal />
      </TablePageLayout>
    </Page>
  );
};
