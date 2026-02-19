import { useMemo, useState } from 'react';
import { Page } from '../../shared/components/Page/Page';
import './style.scss';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { GatewaysTab } from './tabs/GatewaysTab';
import { LocationsTab } from './tabs/LocationsTab';

export const LocationsPageTab = {
  Locations: 'locations',
  Gateways: 'gateways',
} as const;

export type LocationsPageTabValue =
  (typeof LocationsPageTab)[keyof typeof LocationsPageTab];

export const LocationsPage = () => {
  const [activeTab, setActiveTab] = useState<LocationsPageTabValue>(
    LocationsPageTab.Locations,
  );

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        title: 'Locations',
        active: activeTab === LocationsPageTab.Locations,
        onClick: () => {
          setActiveTab(LocationsPageTab.Locations);
        },
      },
      {
        title: 'Gateways',
        active: activeTab === LocationsPageTab.Gateways,
        onClick: () => {
          setActiveTab(LocationsPageTab.Gateways);
        },
      },
    ],
    [activeTab],
  );

  return (
    <Page title="Locations" id="locations-page">
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <TablePageLayout>
        {activeTab === LocationsPageTab.Locations && <LocationsTab />}
        {activeTab === LocationsPageTab.Gateways && <GatewaysTab />}
      </TablePageLayout>
    </Page>
  );
};
