import dayjs from 'dayjs';
import { m } from '../../paraglide/messages';
import {
  LicenseFeature,
  type LicenseFeatureValue,
  type LicenseInfo,
  type LicenseInfoApi,
  LicenseTier,
  type LicenseTierValue,
  SupportType,
  type SupportTypeNarrowValue,
  type SupportTypeValue,
} from '../api/types';
import { isPresent } from '../defguard-ui/utils/isPresent';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';

export type LicenseState =
  | 'noLicense'
  | 'gracePeriod'
  | 'expiredLicense'
  | 'validBusiness'
  | 'validEnterprise';

export interface LicenseCheckResult {
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

const tierIncludedFeatures: Record<LicenseTierValue, LicenseFeatureValue[]> = {
  [LicenseTier.Enterprise]: Object.values(LicenseFeature),
  [LicenseTier.Business]: [],
};

/// Returns only the features that are additive grants (not already covered by the tier
/// baseline). Use this for display; never use it for gating (gating must use `features`).
export const getAdditiveFeatures = (license: LicenseInfo): LicenseFeatureValue[] =>
  license.features.filter((f) => !tierIncludedFeatures[license.tier].includes(f));

export const getLicenseFeatureLabel = (feature: LicenseFeatureValue): string => {
  switch (feature) {
    case LicenseFeature.ComponentHa:
      return m.settings_license_feature_component_ha();
    case LicenseFeature.DevicePosture:
      return m.settings_license_feature_device_posture();
    case LicenseFeature.ServiceLocations:
      return m.settings_license_feature_service_locations();
    case LicenseFeature.AclAllowedIps:
      return m.settings_license_feature_acl_allowed_ips();
    default:
      return feature;
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

// When a specific `feature` is passed, the gate opens if the license grants that feature
// individually (an additive flag). Without a
// `feature`, the check falls back to the strict Enterprise-tier gate.
export const canUseEnterpriseFeature = (
  license: LicenseInfo | null,
  feature?: LicenseFeatureValue,
): LicenseCheckResult => {
  if (!license)
    return {
      error: 'tier',
      result: false,
      tierCheck: 'Enterprise',
    };

  // Check expiry before the grant: the backend clears `features` to `[]` for an expired
  // license while keeping `expired: true`, so a granted-but-expired Enterprise license must
  // surface as 'expired' rather than falling through to the 'tier' (upgrade) path.
  if (license.expired)
    return {
      error: 'expired',
      result: false,
      tierCheck: 'Enterprise',
    };

  const granted = isPresent(feature)
    ? (license.features?.includes(feature) ?? false)
    : license.tier === 'Enterprise';

  if (!granted)
    return {
      error: 'tier',
      result: false,
      tierCheck: 'Enterprise',
    };

  return {
    result: true,
    error: null,
    tierCheck: 'Enterprise',
  };
};

// Shared so the modal and the wizard route guard gate identically.
export const canUseServiceLocations = (license: LicenseInfo | null): boolean =>
  canUseEnterpriseFeature(license, LicenseFeature.ServiceLocations).result;

// A granted `licenseFeature` can satisfy an Enterprise requirement on a lower tier. Return
// undefined when no tier is required.
export const tierLicenseCheck = (
  license: LicenseInfo | null,
  licenseTier: LicenseTierValue | null | undefined,
  licenseFeature?: LicenseFeatureValue,
): LicenseCheckResult | undefined => {
  switch (licenseTier) {
    case LicenseTier.Business:
      return canUseBusinessFeature(license);
    case LicenseTier.Enterprise:
      return canUseEnterpriseFeature(license, licenseFeature);
    default:
      return undefined;
  }
};

export const isNavItemLocked = (
  license: LicenseInfo | null,
  licenseTier: LicenseTierValue | null | undefined,
  licenseFeature?: LicenseFeatureValue,
): boolean => tierLicenseCheck(license, licenseTier, licenseFeature)?.result === false;

export const narrowLicenseSupport = (license: LicenseInfoApi): SupportTypeNarrowValue => {
  switch (license.support_type) {
    case 'Basic':
    case 'BasicEnterprise':
      return 'Basic';
    case 'Direct':
    case 'DirectEnterprise':
      return 'Direct';
    default:
      return 'Free';
  }
};
