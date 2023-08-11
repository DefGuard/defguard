import './style.scss';

import { useState } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { Card } from '../../shared/components/layout/Card/Card';
import { CardTabs } from '../../shared/components/layout/CardTabs/CardTabs';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { EnrollmentTab } from './EnrollmentTab/EnrollmentTab';
import { GeneralTab } from './GeneralTab/GeneralTab';
import { SmtpTab } from './SmtpTab/SmtpTab';
import { SupportTab } from './SupportTab/SupportTab';

enum Tabs {
  General,
  Smtp,
  Enrollment,
  Support,
}

export const SettingsPage = () => {
  const { LL } = useI18nContext();
  const [tab, setTab] = useState(Tabs.General);
  const tabs = [
    {
      key: 1,
      onClick: () => {
        setTab(Tabs.General);
      },
      content: LL.settingsPage.tabs.general(),
      active: tab === Tabs.General,
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
      content: LL.settingsPage.tabs.enrollment(),
      active: tab === Tabs.Enrollment,
    },
    {
      key: 4,
      onClick: () => {
        setTab(Tabs.Support);
      },
      content: LL.settingsPage.tabs.support(),
      active: tab === Tabs.Support,
    },
  ];
  const settings = useAppStore((state) => state.settings);
  return (
    <PageContainer id="settings-page">
      <header>
        <h1>
          {settings?.instance_name} {LL.settingsPage.title()}
        </h1>
      </header>
      <CardTabs tabs={tabs} />
      <Card className="settings-card" hideMobile>
        {tab === Tabs.General && <GeneralTab />}
        {tab === Tabs.Smtp && <SmtpTab />}
        {tab === Tabs.Enrollment && <EnrollmentTab />}
        {tab === Tabs.Support && <SupportTab />}
      </Card>
    </PageContainer>
  );
};
