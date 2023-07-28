import './style.scss';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { deviceBreakpoints } from '../../shared/constants';
import { BrandingCard } from './BrandingCard/BrandingCard';
import { BuiltByCard } from './BuiltByCard/BuiltByCard';
import { ModulesCard } from './ModulesCard/ModulesCard';
import { SupportCard } from './SupportCard/SupportCard';
import { Web3Settings } from './Web3Settings/Web3Settings';
import { useBreakpoint } from 'use-breakpoint';
import { Card } from '../../shared/components/layout/Card/Card';
import { useState } from 'react';
import { CardTabs } from '../../shared/components/layout/CardTabs/CardTabs';

enum Tabs {
  Global,
  Smtp,
}

export const SettingsPage = () => {
  const [tab, setTab] = useState(Tabs.Global);
  const tabs = [
    {
      key: 1, onClick: () => {
        setTab(Tabs.Global)
      },
      content: "Global settings",
      active: tab === Tabs.Global,
    },
    {
      key: 2, onClick: () => {
        setTab(Tabs.Smtp)
      },
      content: "SMTP",
      active: tab === Tabs.Smtp,
    }
  ]
  const settings = useAppStore((state) => state.settings);
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  return (
    <PageContainer id="settings-page">
      <header>
        <h1>
          {settings?.instance_name} {LL.settingsPage.title()}
        </h1>
      </header>
      {breakpoint === 'desktop' && <CardTabs tabs={tabs} />}
      <Card className="settings-card" hideMobile>
        <div className="left">
          <BrandingCard />
          <ModulesCard />
          {/*<DefaultNetworkSelect /> */}
        </div>
        <div className="right">
          <Web3Settings />
          <SupportCard />
          <BuiltByCard />
        </div>
      </Card>
    </PageContainer>
  );
};
