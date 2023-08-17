import './style.scss';

import { useQueryClient } from '@tanstack/react-query';
import { ReactNode, useEffect, useMemo, useState } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';
import { CardTabs } from '../../shared/defguard-ui/components/Layout/CardTabs/CardTabs';
import { CardTabsData } from '../../shared/defguard-ui/components/Layout/CardTabs/types';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { QueryKeys } from '../../shared/queries';
import { GlobalSettings } from './components/GlobalSettings/GlobalSettings';
import { SmtpSettings } from './components/SmtpSettings/SmtpSettings';

const tabsContent: ReactNode[] = [<GlobalSettings key={0} />, <SmtpSettings key={1} />];

export const SettingsPage = () => {
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const [activeCard, setActiveCard] = useState(0);

  const tabs = useMemo(
    (): CardTabsData[] => [
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
    ],
    [LL.settingsPage.tabs, activeCard],
  );

  const settings = useAppStore((state) => state.settings);

  // Refetch settings on page mount
  useEffect(() => {
    queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
    // eslint-disable-next-line
  }, []);

  if (!settings) return null;

  return (
    <PageContainer id="settings-page">
      <h1>{LL.settingsPage.title()}</h1>
      <CardTabs tabs={tabs} />
      <Card className="settings-card" hideMobile shaded>
        {tabsContent[activeCard]}
      </Card>
    </PageContainer>
  );
};
