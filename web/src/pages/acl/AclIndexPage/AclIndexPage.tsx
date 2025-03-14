import './style.scss';

import { useMemo, useState } from 'react';

import { PageLayout } from '../../../shared/components/Layout/PageLayout/PageLayout';
import { CardTabs } from '../../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../../shared/defguard-ui/components/Layout/CardTabs/types';
import { AclIndexAliases } from './components/AclIndexAliases/AclIndexAliases';
import { AclIndexRules } from './components/AclIndexRules/AclIndexRules';

enum AclTab {
  ALIASES = 'aliases',
  RULES = 'rules',
}

export const AclIndexPage = () => {
  const [activeTab, setActiveTab] = useState(AclTab.ALIASES);

  const availableTabs: CardTabsData[] = [
    {
      key: AclTab.RULES,
      active: activeTab === AclTab.RULES,
      content: <p>Rules</p>,
      onClick: () => setActiveTab(AclTab.RULES),
    },
    {
      key: AclTab.ALIASES,
      active: activeTab === AclTab.ALIASES,
      content: <p>Aliases</p>,
      onClick: () => setActiveTab(AclTab.ALIASES),
    },
  ];

  const tabRender = useMemo(() => {
    switch (activeTab) {
      case AclTab.RULES:
        return <AclIndexRules />;
      case AclTab.ALIASES:
        return <AclIndexAliases />;
    }
  }, [activeTab]);

  return (
    <PageLayout id="acl-index-page">
      <header>
        <h1>Access Control List</h1>
      </header>
      <CardTabs tabs={availableTabs} />
      <div id="content-card">{tabRender}</div>
    </PageLayout>
  );
};
