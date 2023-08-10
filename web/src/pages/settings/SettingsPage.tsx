import './style.scss';

import { useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { deviceBreakpoints } from '../../shared/constants';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { BrandingCard } from './BrandingCard/BrandingCard';
import { BuiltByCard } from './BuiltByCard/BuiltByCard';
import { EnrollmentTab } from './EnrollmentTab/EnrollmentTab';
import { ModulesCard } from './ModulesCard/ModulesCard';
import { SmtpCard } from './SmtpCard/SmtpCard';
import { SupportCard } from './SupportCard/SupportCard';
import { Web3Settings } from './Web3Settings/Web3Settings';

enum Tabs {
  Basic,
  Smtp,
  Enrollment,
}

export const SettingsPage = () => {
  const { LL } = useI18nContext();
  const [tab, setTab] = useState(Tabs.Basic);
  const tabs = [
    {
      key: 1,
      onClick: () => {
        setTab(Tabs.Basic);
      },
      content: LL.settingsPage.tabs.basic(),
      active: tab === Tabs.Basic,
    },
    {
      key: 2,
      onClick: () => {
        setTab(Tabs.Smtp);
      },
      content: LL.settingsPage.tabs.smtp(),
      active: tab === Tabs.Smtp,
    },
    {
      key: 3,
      onClick: () => {
        setTab(Tabs.Enrollment);
      },
      content: 'Enrollment',
      active: tab === Tabs.Enrollment,
    },
  ];
  const settings = useAppStore((state) => state.settings);
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
        {tab === Tabs.Basic && (
          <>
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
          </>
        )}
        {tab === Tabs.Smtp && <SmtpCard />}
        {tab === Tabs.Enrollment && <EnrollmentTab />}
      </Card>
    </PageContainer>
  );
};
