import { createFileRoute } from '@tanstack/react-router';
import { NetworkDevicesPage } from '../../../pages/NetworkDevicesPage/NetworkDevicesPage';
import { getNetworkDevicesQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/network-devices')({
  component: NetworkDevicesPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getNetworkDevicesQueryOptions);
  },
});
