import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getNetworkDevicesQueryOptions } from '../../shared/query';
import { AddNetworkDeviceModal } from './modals/AddNetworkDeviceModal/AddNetworkDeviceModal';
import { EditNetworkDeviceModal } from './modals/EditNetworkDeviceModal/EditNetworkDeviceModal';
import { NetworkDeviceConfigModal } from './modals/NetworkDeviceConfigModal/NetworkDeviceConfigModal';
import { NetworkDeviceTokenModal } from './modals/NetworkDeviceTokenModal/NetworkDeviceTokenModal';
import { NetworkDevicesTable } from './NetworkDevicesTable';

export const NetworkDevicesPage = () => {
  const { data: networkDevices } = useSuspenseQuery(getNetworkDevicesQueryOptions);

  return (
    <>
      <Page id="network-devices-page" title="Network Devices">
        <TablePageLayout>
          <NetworkDevicesTable networkDevices={networkDevices} />
        </TablePageLayout>
      </Page>
      <AddNetworkDeviceModal />
      <NetworkDeviceConfigModal />
      <NetworkDeviceTokenModal />
      <EditNetworkDeviceModal />
    </>
  );
};
