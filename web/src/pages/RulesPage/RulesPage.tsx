import { useQuery } from '@tanstack/react-query';
import { Suspense, useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
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
  const [activeTab, setActiveTab] = useState<RulesPageTabValue>(RulesPageTab.Deployed);

  const { data: rulesCount } = useQuery(getRulesCountQueryOptions);

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
        onClick: () => {
          setActiveTab(RulesPageTab.Deployed);
        },
      },
      {
        title: pendingTabTitle,
        icon: pendingIcon,
        active: activeTab === RulesPageTab.Pending,
        onClick: () => {
          setActiveTab(RulesPageTab.Pending);
        },
      },
    ],
    [activeTab, pendingIcon, pendingTabTitle],
  );

  return (
    <Page title={m.cmp_nav_item_rules()} id="rules-page">
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
