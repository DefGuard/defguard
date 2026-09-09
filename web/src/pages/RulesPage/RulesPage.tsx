import { useQuery } from '@tanstack/react-query';
import { useNavigate, useSearch } from '@tanstack/react-router';
import { Suspense, useCallback, useEffect, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import { getCanonicalAclListUrlSearch } from '../../shared/aclTabs';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getRulesCountQueryOptions } from '../../shared/query';
import { RulesDeployedTab } from './tabs/RulesDeployedTab';
import { RulesPendingTab } from './tabs/RulesPendingTab';
import { RulesPageTab, type RulesPageTabValue } from './types';

export const RulesPage = () => {
  const navigate = useNavigate({ from: '/acl/rules' });
  const search = useSearch({ from: '/_authorized/_default/acl/rules' });
  const activeTab = search.tab;

  useEffect(() => {
    if (window.location.search === getCanonicalAclListUrlSearch(activeTab)) {
      return;
    }

    void navigate({ search: { tab: activeTab }, replace: true });
  }, [activeTab, navigate]);

  const { data: rulesCount } = useQuery(getRulesCountQueryOptions);

  const setActiveTab = useCallback(
    (tab: RulesPageTabValue) => {
      navigate({ search: { tab } });
    },
    [navigate],
  );

  const pendingCount = rulesCount?.pending ?? 0;
  const pendingTabTitle = useMemo(
    () =>
      pendingCount > 0 ? `${m.state_pending()} (${pendingCount})` : m.state_pending(),
    [pendingCount],
  );
  const pendingIcon = pendingCount > 0 ? IconKind.AttentionFilled : undefined;

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        title: m.state_deployed(),
        active: activeTab === RulesPageTab.Deployed,
        onClick: () => setActiveTab(RulesPageTab.Deployed),
        testId: 'rules-tab-deployed',
      },
      {
        title: pendingTabTitle,
        icon: pendingIcon,
        active: activeTab === RulesPageTab.Pending,
        onClick: () => setActiveTab(RulesPageTab.Pending),
        testId: 'rules-tab-pending',
      },
    ],
    [activeTab, pendingIcon, pendingTabTitle, setActiveTab],
  );

  return (
    <Page title={m.cmp_nav_item_rules()} id="rules-page">
      <TablePageLayout>
        <Tabs items={tabs} />
        <Suspense fallback={<TableSkeleton />}>
          {activeTab === RulesPageTab.Deployed && <RulesDeployedTab />}
          {activeTab === RulesPageTab.Pending && <RulesPendingTab />}
        </Suspense>
      </TablePageLayout>
    </Page>
  );
};
