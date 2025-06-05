import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ReactNode, useCallback, useEffect, useMemo, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { useUpgradeLicenseModal } from '../../shared/components/Layout/UpgradeLicenseModal/store';
import { UpgradeLicenseModalVariant } from '../../shared/components/Layout/UpgradeLicenseModal/types';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../shared/defguard-ui/components/Layout/CardTabs/types';
import { LoaderSpinner } from '../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { ActivityStreamSettings } from './components/ActivityStreamSettings/ActivityStreamSettings';
import { EnterpriseSettings } from './components/EnterpriseSettings/EnterpriseSettings';
import { GlobalSettings } from './components/GlobalSettings/GlobalSettings';
import { LdapSettings } from './components/LdapSettings/LdapSettings';
import { NotificationSettings } from './components/NotificationSettings/NotificationSettings';
import { OpenIdSettings } from './components/OpenIdSettings/OpenIdSettings';
import { SmtpSettings } from './components/SmtpSettings/SmtpSettings';
import { useSettingsPage } from './hooks/useSettingsPage';

const tabsContent: ReactNode[] = [
  <GlobalSettings key={0} />,
  <SmtpSettings key={1} />,
  <LdapSettings key={2} />,
  <OpenIdSettings key={3} />,
  <EnterpriseSettings key={4} />,
  <NotificationSettings key={5} />,
  <ActivityStreamSettings key={6} />,
];

const enterpriseTabs: number[] = [2, 3, 4, 6];

export const SettingsPage = () => {
  const { LL } = useI18nContext();
  const {
    getEnterpriseInfo,
    settings: { getSettings },
  } = useApi();

  const [activeCard, setActiveCard] = useState(0);
  const queryClient = useQueryClient();
  const appInfo = useAppStore((s) => s.appInfo);
  const openUpgradeLicenseModal = useUpgradeLicenseModal((s) => s.open, shallow);

  const [setPageState, resetPageState] = useSettingsPage(
    (state) => [state.setState, state.reset],
    shallow,
  );

  const settings = useSettingsPage((state) => state.settings);

  const { data: settingsData, isLoading: settingsLoading } = useQuery({
    queryFn: getSettings,
    queryKey: [QueryKeys.FETCH_SETTINGS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  const { data: enterpriseInfo, isLoading: enterpriseInfoLoading } = useQuery({
    queryFn: getEnterpriseInfo,
    queryKey: [QueryKeys.FETCH_ENTERPRISE_INFO],
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const handleTabClick = useCallback(
    (tabIndex: number) => {
      if (appInfo) {
        if (enterpriseTabs.includes(tabIndex) && !appInfo.license_info.enterprise) {
          openUpgradeLicenseModal({
            modalVariant: UpgradeLicenseModalVariant.ENTERPRISE_NOTICE,
          });
        } else {
          setActiveCard(tabIndex);
        }
      }
    },
    [appInfo, openUpgradeLicenseModal],
  );

  const tabs = useMemo(
    (): CardTabsData[] => [
      {
        key: 0,
        content: LL.settingsPage.tabs.global(),
        active: activeCard === 0,
        onClick: () => handleTabClick(0),
      },
      {
        key: 1,
        content: LL.settingsPage.tabs.smtp(),
        active: activeCard === 1,
        onClick: () => handleTabClick(1),
      },
      {
        key: 2,
        content: LL.settingsPage.tabs.ldap(),
        active: activeCard === 2,
        onClick: () => handleTabClick(2),
      },
      {
        key: 3,
        content: LL.settingsPage.tabs.openid(),
        active: activeCard === 3,
        onClick: () => handleTabClick(3),
      },
      {
        key: 4,
        content: LL.settingsPage.tabs.enterprise(),
        active: activeCard === 4,
        onClick: () => handleTabClick(4),
      },
      {
        key: 5,
        content: LL.settingsPage.tabs.gatewayNotifications(),
        active: activeCard === 5,
        onClick: () => handleTabClick(5),
      },
      {
        key: 6,
        content: LL.settingsPage.tabs.activityStream(),
        active: activeCard === 6,
        onClick: () => handleTabClick(6),
      },
    ],
    [LL.settingsPage.tabs, activeCard, handleTabClick],
  );

  // set store
  useEffect(() => {
    setPageState({
      settings: settingsData,
      enterpriseInfo: enterpriseInfo?.license_info,
    });
  }, [settingsData, setPageState, enterpriseInfo?.license_info]);

  useEffect(() => {
    void queryClient.invalidateQueries({
      queryKey: [QueryKeys.FETCH_APP_INFO],
    });
    return () => {
      resetPageState?.();
    };
    // eslint-disable-next-line
  }, []);

  // if appinfo changes and license is not enterprise anymore then change active tab to global
  // this can happen when admin is on enterprise tab but limits are exceeded in the mean time
  useEffect(() => {
    if (
      appInfo &&
      !appInfo.license_info.enterprise &&
      enterpriseTabs.includes(activeCard)
    ) {
      setActiveCard(0);
      openUpgradeLicenseModal({
        modalVariant: UpgradeLicenseModalVariant.LICENSE_LIMIT,
      });
    }
  }, [activeCard, appInfo, openUpgradeLicenseModal]);

  return (
    <PageContainer id="settings-page">
      <h1>{LL.settingsPage.title()}</h1>
      {(settingsLoading || enterpriseInfoLoading) && <LoaderSpinner size={250} />}
      {settings && !enterpriseInfoLoading && !settingsLoading && (
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
