import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { ReactNode, useEffect, useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../shared/defguard-ui/components/Layout/CardTabs/types';
import { LoaderSpinner } from '../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { GlobalSettings } from './components/GlobalSettings/GlobalSettings';
import { LdapSettings } from './components/LdapSettings/LdapSettings';
import { OpenIdSettings } from './components/OpenIdSettings/OpenIdSettings';
import { SmtpSettings } from './components/SmtpSettings/SmtpSettings';
import { useSettingsPage } from './hooks/useSettingsPage';

const tabsContent: ReactNode[] = [
  <GlobalSettings key={0} />,
  <SmtpSettings key={1} />,
  <LdapSettings key={2} />,
  <OpenIdSettings key={3} />,
];

export const SettingsPage = () => {
  const { LL } = useI18nContext();
  const {
    settings: { getSettings },
  } = useApi();

  const [activeCard, setActiveCard] = useState(0);

  const [setPageState, resetPageState] = useSettingsPage(
    (state) => [state.setState, state.reset],
    shallow,
  );

  const settings = useSettingsPage((state) => state.settings);

  const enterpriseEnabled = useAppStore((state) => state.enterprise_enabled);

  const { data: settingsData, isLoading } = useQuery({
    queryFn: getSettings,
    queryKey: [QueryKeys.FETCH_SETTINGS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  const tabs = useMemo((): CardTabsData[] => {
    let tabs = [
      {
        key: 0,
        content: LL.settingsPage.tabs.global(),
        active: activeCard === 0,
        onClick: () => setActiveCard(0),
      },
      {
        key: 1,
        content: LL.settingsPage.tabs.smtp(),
        active: activeCard === 1,
        onClick: () => setActiveCard(1),
      },
      {
        key: 2,
        content: LL.settingsPage.tabs.ldap(),
        active: activeCard === 2,
        onClick: () => setActiveCard(2),
      },
      {
        key: 3,
        content: LL.settingsPage.tabs.openid(),
        active: activeCard === 3,
        onClick: () => setActiveCard(3),
      },
    ];

    // Fitler out enterprise tabs if not enterprise
    if (!enterpriseEnabled) {
      tabs = tabs.filter((tab) => tab.key !== 3);
    }

    return tabs;
  }, [LL.settingsPage.tabs, activeCard, enterpriseEnabled]);

  // set store
  useEffect(() => {
    if (settingsData) {
      setPageState({ settings: settingsData });
    }
  }, [settingsData, setPageState]);

  useEffect(() => {
    return () => {
      resetPageState?.();
    };
    // eslint-disable-next-line
  }, []);

  return (
    <PageContainer id="settings-page">
      <h1>{LL.settingsPage.title()}</h1>
      {!settingsData && isLoading && <LoaderSpinner size={250} />}
      {settings && (
        <>
          <CardTabs tabs={tabs} />
          <Card className="settings-card" hideMobile shaded>
            {tabsContent[activeCard]}
          </Card>
        </>
      )}
    </PageContainer>
  );
};
