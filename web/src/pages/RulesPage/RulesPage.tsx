import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { AclStatus } from '../../shared/api/types';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getRulesQueryOptions } from '../../shared/query';
import { RulesDeployedTab } from './tabs/RulesDeployedTab';
import { RulesPendingTab } from './tabs/RulesPendingTab';
import { RulesPageTab, type RulesPageTabValue } from './types';

export const RulesPage = () => {
  // FIXME: split into separate queries
  const { data: rules } = useQuery(getRulesQueryOptions);

  const deployed = useMemo(() => {
    if (isPresent(rules)) {
      return rules.filter((rule) => rule.state === AclStatus.Applied);
    }
  }, [rules]);

  const pending = useMemo(() => {
    if (isPresent(rules)) {
      return rules.filter((rule) => rule.state !== AclStatus.Applied);
    }
  }, [rules]);

  const [activeTab, setActiveTab] = useState<RulesPageTabValue>(RulesPageTab.Deployed);

  const pendingTabTitle = useMemo(
    () => `Pending${pending?.length ? ` (${pending.length})` : ''}`,
    [pending],
  );
  const tabs = useMemo(
    (): TabsItem[] => [
      {
        title: 'Deployed',
        active: activeTab === RulesPageTab.Deployed,
        onClick: () => {
          setActiveTab(RulesPageTab.Deployed);
        },
      },
      {
        title: pendingTabTitle,
        active: activeTab === RulesPageTab.Pending,
        onClick: () => {
          setActiveTab(RulesPageTab.Pending);
        },
      },
    ],
    [activeTab, pendingTabTitle],
  );

  return (
    <Page title="Rules" id="rules-page">
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <TablePageLayout>
        {activeTab === RulesPageTab.Deployed && isPresent(deployed) && (
          <RulesDeployedTab rules={deployed} />
        )}
        {activeTab === RulesPageTab.Pending && isPresent(pending) && (
          <RulesPendingTab rules={pending} />
        )}
      </TablePageLayout>
    </Page>
  );
};
