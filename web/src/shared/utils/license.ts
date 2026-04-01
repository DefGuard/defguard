import dayjs from 'dayjs';
import { m } from '../../paraglide/messages';
import { type LicenseInfo, SupportType, type SupportTypeValue } from '../api/types';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';

export type LicenseState =
  | 'noLicense'
  | 'gracePeriod'
  | 'expiredLicense'
  | 'validBusiness'
  | 'validEnterprise';

interface LicenseCheckResult {
  result: boolean;
  error: 'expired' | 'tier' | null;
  tierCheck: 'Business' | 'Enterprise';
}

export const getLicenseState = (
  licenseInfo: LicenseInfo | null | undefined,
): LicenseState | null => {
  if (licenseInfo === undefined) {
    return null;
  }

  if (licenseInfo === null) {
    return 'noLicense';
  }

  if (licenseInfo.expired) {
    return 'expiredLicense';
  }

  if (
    licenseInfo.subscription &&
    licenseInfo.valid_until !== null &&
    dayjs().isAfter(dayjs.utc(licenseInfo.valid_until).local())
  ) {
    return 'gracePeriod';
  }

  if (licenseInfo.tier === 'Enterprise') {
    return 'validEnterprise';
  }

  return 'validBusiness';
};

export const getSupportTypeLabel = (supportType: SupportTypeValue): string => {
  switch (supportType) {
    case SupportType.Free:
      return m.license_support_type_free();
    case SupportType.Basic:
      return m.license_support_type_basic();
    case SupportType.Direct:
      return m.license_support_type_direct();
    case SupportType.BasicEnterprise:
      return m.license_support_type_basic_enterprise();
    case SupportType.DirectEnterprise:
      return m.license_support_type_direct_enterprise();
    default:
      return supportType;
  }
};

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
