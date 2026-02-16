import { createFileRoute, redirect } from '@tanstack/react-router';
import { AddLocationPage } from '../../../pages/AddLocationPage/AddLocationPage';
import { useAddLocationStore } from '../../../pages/AddLocationPage/useAddLocationStore';
import api from '../../../shared/api/api';
import { type LicenseInfo, LicenseTier } from '../../../shared/api/types';
import { getLicenseInfoQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_wizard/add-location')({
  component: AddLocationPage,
  beforeLoad: async ({ context }) => {
    const isServiceWizard = useAddLocationStore.getState().locationType === 'service';
    if (isServiceWizard) {
      let licenseInfo: LicenseInfo | null = null;
      const cachedLicense = context.queryClient.getQueryData(
        getLicenseInfoQueryOptions.queryKey,
      )?.data;
      if (!cachedLicense) {
        licenseInfo = (await api.getLicenseInfo()).data.license_info;
      } else {
        licenseInfo = cachedLicense.license_info;
      }
      if (licenseInfo?.tier !== LicenseTier.Enterprise) {
        throw redirect({ to: '/locations', replace: true });
      }
    }
  },
});
