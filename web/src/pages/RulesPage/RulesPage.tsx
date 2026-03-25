import { useQuery } from '@tanstack/react-query';
import { useNavigate, useSearch } from '@tanstack/react-router';
import { Suspense, useCallback, useEffect, useMemo } from 'react';
import { getCanonicalAclListUrlSearch } from '../../shared/aclTabs';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
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
    () => `Pending${pendingCount ? ` (${pendingCount})` : ''}`,
    [pendingCount],
  );
  const pendingIcon = pendingCount > 0 ? IconKind.AttentionFilled : undefined;

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        title: 'Deployed',
        active: activeTab === RulesPageTab.Deployed,
        onClick: () => setActiveTab(RulesPageTab.Deployed),
      },
      {
        title: pendingTabTitle,
        icon: pendingIcon,
        active: activeTab === RulesPageTab.Pending,
        onClick: () => setActiveTab(RulesPageTab.Pending),
      },
    ],
    [activeTab, pendingIcon, pendingTabTitle, setActiveTab],
  );

  return (
    <Page title="Rules" id="rules-page">
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Suspense fallback={<TableSkeleton />}>
        <TablePageLayout>
          {activeTab === RulesPageTab.Deployed && <RulesDeployedTab />}
          {activeTab === RulesPageTab.Pending && <RulesPendingTab />}
        </TablePageLayout>
      </Suspense>
    </Page>
  );
};
