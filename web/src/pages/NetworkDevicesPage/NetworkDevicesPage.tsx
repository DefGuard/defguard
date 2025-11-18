import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { getNetworkDevicesQueryOptions } from '../../shared/query';
import { AddNetworkDeviceModal } from './modals/AddNetworkDeviceModal/AddNetworkDeviceModal';
import { NetworkDevicesTable } from './NetworkDevicesTable';

export const NetworkDevicesPage = () => {
  const { data: networkDevices } = useSuspenseQuery(getNetworkDevicesQueryOptions);

  return (
    <>
      <Page id="network-devices-page" title="Network Devices">
        <NetworkDevicesTable networkDevices={networkDevices} />
      </Page>
      <AddNetworkDeviceModal />
    </>
  );
};
