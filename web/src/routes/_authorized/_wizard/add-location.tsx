import { createFileRoute, redirect } from '@tanstack/react-router';
import { AddLocationPage } from '../../../pages/AddLocationPage/AddLocationPage';
import { useAddLocationStore } from '../../../pages/AddLocationPage/useAddLocationStore';
import { LicenseTier } from '../../../shared/api/types';
import { getLicenseInfoQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_wizard/add-location')({
  component: AddLocationPage,
  beforeLoad: async ({ context }) => {
    const isServiceWizard = useAddLocationStore.getState().locationType === 'service';
    if (isServiceWizard) {
      const licenseInfo = await context.queryClient.fetchQuery(
        getLicenseInfoQueryOptions,
      );
      if (licenseInfo?.tier !== LicenseTier.Enterprise) {
        throw redirect({ to: '/locations', replace: true });
      }
    }
  },
});
