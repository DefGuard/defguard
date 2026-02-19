import { Suspense, useMemo, useState } from 'react';
import { Page } from '../../shared/components/Page/Page';
import { LocationsTable } from './components/LocationsTable';
import { AddLocationModal } from './modals/AddLocationModal/AddLocationModal';
import './style.scss';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';

export const LocationsPageTab = {
  Locations: 'locations',
  Gateways: 'gateways',
} as const;

export type LocationsPageTabValue = (typeof LocationsPageTab)[keyof typeof LocationsPageTab];

export const LocationsPage = () => {
  const [activeTab, setActiveTab] = useState<LocationsPageTabValue>(LocationsPageTab.Locations);

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
    [activeTab ],
  );
  return (
    <>
      <Page title="Locations" id="locations-page">
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      <SizedBox height={ThemeSpacing.Xl2} />
        <TablePageLayout>
          <Suspense fallback={<TableSkeleton />}>
            <LocationsTable />
          </Suspense>
        </TablePageLayout>
      </Page>
      <AddLocationModal />
    </>
  );
};
