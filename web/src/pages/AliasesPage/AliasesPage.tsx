import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate, useSearch } from '@tanstack/react-router';
import { Suspense, useCallback, useEffect, useMemo } from 'react';
import { m } from '../../paraglide/messages';
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
import { getAliasesCountQueryOptions } from '../../shared/query';
import { DeleteAliasDestinationConfirmModal } from '../Acl/components/DeleteAliasDestinationConfirmModal/DeleteAliasDestinationConfirmModal';
import { AliasesDeployedTab } from './tabs/AliasesDeployedTab';
import { AliasesPendingTab } from './tabs/AliasesPendingTab';

export const AliasesPage = () => {
  const navigate = useNavigate({ from: '/acl/aliases' });
  const search = useSearch({ from: '/_authorized/_default/acl/aliases' });
  const activeTab = search.tab;

  useEffect(() => {
    if (window.location.search === getCanonicalAclListUrlSearch(activeTab)) {
      return;
    }

    void navigate({ search: { tab: activeTab }, replace: true });
  }, [activeTab, navigate]);

  const { data: aliasesCount } = useQuery(getAliasesCountQueryOptions);

  const setActiveTab = useCallback(
    (tab: AclListTabValue) => {
      navigate({ search: { tab } });
    },
    [navigate],
  );

  const pendingCount = aliasesCount?.pending ?? 0;
  const pendingTitle = pendingCount
    ? `${m.state_pending()} (${pendingCount})`
    : m.state_pending();
  const pendingIcon = pendingCount > 0 ? IconKind.AttentionFilled : undefined;

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        active: activeTab === AclListTab.Deployed,
        onClick: () => setActiveTab(AclListTab.Deployed),
        title: m.state_deployed(),
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
    <Page id="aliases-page" title={m.cmp_nav_item_aliases()}>
      <TablePageLayout>
        <Tabs items={tabs} />
        <Suspense fallback={<TableSkeleton />}>
          {activeTab === AclListTab.Deployed && <AliasesDeployedTab />}
          {activeTab === AclListTab.Pending && <AliasesPendingTab />}
        </Suspense>
        <DeleteAliasDestinationConfirmModal />
      </TablePageLayout>
    </Page>
  );
};
