import { useSuspenseQuery } from '@tanstack/react-query';
import { getLicenseInfoQueryOptions } from '../../../shared/query';
import { canUseEnterpriseFeature } from '../../../shared/utils/license';

export const useEnterpriseTier = () => {
  const { data: licenseInfo } = useSuspenseQuery(getLicenseInfoQueryOptions);
  return canUseEnterpriseFeature(licenseInfo).result;
};
