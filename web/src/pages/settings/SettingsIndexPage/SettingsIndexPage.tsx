import { useNavigate, useSearch } from '@tanstack/react-router';
import { type JSX, useMemo } from 'react';
import { Page } from '../../../shared/components/Page/Page';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabProps } from '../../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { SettingsExternalOpenIdPage } from '../SettingsExternalOpenIdPage/SettingsExternalOpenIdPage';
import { SettingsGeneralTab } from './tabs/SettingsGeneralTab';
import { SettingsLicenseTab } from './tabs/SettingsLicenseTab/SettingsLicenseTab';
import { SettingsNotificationsTab } from './tabs/SettingsNotificationsTab';
import { type SettingsTabValue, settingsTabsSchema } from './types';

const ActivityTab = () => null;

const tabComponent: Record<SettingsTabValue, JSX.Element> = {
  general: <SettingsGeneralTab />,
  notifications: <SettingsNotificationsTab />,
  openid: <SettingsExternalOpenIdPage />,
  activity: <ActivityTab />,
  license: <SettingsLicenseTab />,
};

const tabToTitle = (tab: SettingsTabValue): string => {
  switch (tab) {
    case 'general':
      return 'General';
    case 'activity':
      return 'Activity streaming';
    case 'license':
      return 'License';
    case 'notifications':
      return 'Notifications';
    case 'openid':
      return 'External identity providers';
  }
};

export const SettingsIndexPage = () => {
  const navigateTab = useNavigate({ from: '/settings' });
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
    <Page id="settings-index-page" title="Settings">
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      {tabComponent[search.tab]}
    </Page>
  );
};
