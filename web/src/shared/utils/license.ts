import type { LicenseInfo } from '../api/types';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';

interface LicenseCheckResult {
  result: boolean;
  error: 'expired' | 'tier' | null;
  tierCheck: 'Business' | 'Enterprise';
}

export const licenseActionCheck = (
  checkResult: LicenseCheckResult,
  successCallback: () => void,
) => {
  const { result, error, tierCheck } = checkResult;
  if (result) {
    successCallback();
  } else {
    switch (error) {
      case 'expired':
        openModal(ModalName.LicenseExpired, {
          licenseTier: tierCheck,
        });
        break;
      case 'tier':
        switch (tierCheck) {
          case 'Business':
            openModal(ModalName.UpgradeBusiness);
            break;
          case 'Enterprise':
            openModal(ModalName.UpgradeEnterprise);
            break;
        }
        break;
    }
  }
};

export const canUseBusinessFeature = (
  license: LicenseInfo | null,
): LicenseCheckResult => {
  if (!license)
    return {
      error: 'tier',
      result: false,
      tierCheck: 'Business',
    };
  if (license.expired)
    return {
      error: 'expired',
      result: false,
      tierCheck: 'Business',
    };
  return {
    result: true,
    error: null,
    tierCheck: 'Business',
  };
};

export const canUseEnterpriseFeature = (
  license: LicenseInfo | null,
): LicenseCheckResult => {
  if (!license || license.tier !== 'Enterprise')
    return {
      error: 'tier',
      result: false,
      tierCheck: 'Enterprise',
    };

  if (license.expired)
    return {
      error: 'expired',
      result: false,
      tierCheck: 'Enterprise',
    };

  return {
    result: true,
    error: null,
    tierCheck: 'Enterprise',
  };
};
