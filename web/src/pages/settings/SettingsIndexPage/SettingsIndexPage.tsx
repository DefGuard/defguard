import { useNavigate, useSearch } from '@tanstack/react-router';
import { type JSX, useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import { Page } from '../../../shared/components/Page/Page';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabProps } from '../../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { SettingsActivityLogStreamingPage } from '../SettingsActivityLogStreamingPage/SettingsActivityStreamingTab';
import { SettingsExternalProvidersTab } from './tabs/SettingsExternalProvidersTab';
import { SettingsGeneralTab } from './tabs/SettingsGeneralTab';
import { SettingsLicenseTab } from './tabs/SettingsLicenseTab/SettingsLicenseTab';
import { SettingsNotificationsTab } from './tabs/SettingsNotificationsTab';
import { type SettingsTabValue, settingsTabsSchema } from './types';
import { SettingsCertificatesTab } from './tabs/SettingsCertificatesTab/SettingsCertificatesTab';

const tabComponent: Record<SettingsTabValue, JSX.Element> = {
  general: <SettingsGeneralTab />,
  notifications: <SettingsNotificationsTab />,
  activity: <SettingsActivityLogStreamingPage />,
  license: <SettingsLicenseTab />,
  identity: <SettingsExternalProvidersTab />,
  certs: <SettingsCertificatesTab />,
};

const tabToTitle = (tab: SettingsTabValue): string => {
  switch (tab) {
    case 'general':
      return m.settings_tab_general();
    case 'activity':
      return m.settings_tab_activity_streaming();
    case 'license':
      return m.settings_tab_license();
    case 'notifications':
      return m.settings_tab_notifications();
    case 'identity':
      return m.settings_tab_identity_providers();
    case 'certs':
      return m.settings_tab_certificates();
  }
};

export const SettingsIndexPage = () => {
  const navigateTab = useNavigate({ from: '/settings/' });
  const search = useSearch({ from: '/_authorized/_default/settings/' });

  const tabs: TabProps[] = useMemo(
    () =>
      settingsTabsSchema.options.map(
        (tab): TabProps => ({
          title: tabToTitle(tab),
          active: search.tab === tab,
          onClick: () => {
            navigateTab({ search: { tab } });
          },
        }),
      ),
    [navigateTab, search.tab],
  );

  return (
    <Page id="settings-index-page" title={m.settings_page_title()}>
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      {tabComponent[search.tab]}
    </Page>
  );
};
