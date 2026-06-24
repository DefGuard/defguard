import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LicenseFeature, type LicenseInfo, LicenseTier } from '../src/shared/api/types';
import { openModal } from '../src/shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../src/shared/hooks/modalControls/modalTypes';
import {
  canUseBusinessFeature,
  canUseEnterpriseFeature,
  canUseServiceLocations,
  getAdditiveFeatures,
  getLicenseState,
  isNavItemLocked,
  licenseActionCheck,
} from '../src/shared/utils/license';

vi.mock('../src/shared/hooks/modalControls/modalsSubjects', () => ({
  openModal: vi.fn(),
}));

dayjs.extend(utc);

const makeLicense = (overrides: Partial<LicenseInfo> = {}): LicenseInfo => ({
  subscription: false,
  valid_until: null,
  expired: false,
  limits_exceeded: false,
  tier: 'Business',
  limits: null,
  features: [],
  support_type: 'Free',
  support_type_narrow: 'Free',
  ...overrides,
});

describe('getLicenseState', () => {
  it('should return null for undefined (not yet loaded)', () => {
    expect(getLicenseState(undefined)).toBeNull();
  });

  it('should return noLicense for null', () => {
    expect(getLicenseState(null)).toBe('noLicense');
  });

  it('should return expiredLicense when expired flag is set', () => {
    expect(getLicenseState(makeLicense({ expired: true }))).toBe('expiredLicense');
  });

  it('should return validBusiness for valid Business license', () => {
    expect(getLicenseState(makeLicense({ tier: 'Business' }))).toBe('validBusiness');
  });

  it('should return validEnterprise for valid Enterprise license', () => {
    expect(getLicenseState(makeLicense({ tier: 'Enterprise' }))).toBe('validEnterprise');
  });

  it('should return gracePeriod for subscription license past valid_until', () => {
    const pastDate = '2000-01-01T00:00:00Z';
    const license = makeLicense({
      subscription: true,
      valid_until: pastDate,
      expired: false,
    });
    expect(getLicenseState(license)).toBe('gracePeriod');
  });

  it('should return validBusiness for subscription license before valid_until', () => {
    const futureDate = '2099-01-01T00:00:00Z';
    const license = makeLicense({
      subscription: true,
      valid_until: futureDate,
      expired: false,
      tier: 'Business',
    });
    expect(getLicenseState(license)).toBe('validBusiness');
  });

  it('should return expiredLicense before checking gracePeriod (expired takes precedence)', () => {
    const pastDate = '2000-01-01T00:00:00Z';
    const license = makeLicense({
      subscription: true,
      valid_until: pastDate,
      expired: true,
    });
    expect(getLicenseState(license)).toBe('expiredLicense');
  });
});

describe('canUseBusinessFeature', () => {
  it('should allow access with valid Business license', () => {
    const result = canUseBusinessFeature(makeLicense({ tier: 'Business' }));
    expect(result.result).toBe(true);
    expect(result.error).toBeNull();
    expect(result.tierCheck).toBe('Business');
  });

  it('should allow access with valid Enterprise license', () => {
    const result = canUseBusinessFeature(makeLicense({ tier: 'Enterprise' }));
    expect(result.result).toBe(true);
    expect(result.error).toBeNull();
  });

  it('should deny access when no license (null)', () => {
    const result = canUseBusinessFeature(null);
    expect(result.result).toBe(false);
    expect(result.error).toBe('tier');
  });

  it('should deny access when license is expired', () => {
    const result = canUseBusinessFeature(makeLicense({ expired: true }));
    expect(result.result).toBe(false);
    expect(result.error).toBe('expired');
  });
});

describe('canUseEnterpriseFeature', () => {
  it('should allow access with valid Enterprise license', () => {
    const result = canUseEnterpriseFeature(makeLicense({ tier: 'Enterprise' }));
    expect(result.result).toBe(true);
    expect(result.error).toBeNull();
    expect(result.tierCheck).toBe('Enterprise');
  });

  it('should deny access when license is Business tier', () => {
    const result = canUseEnterpriseFeature(makeLicense({ tier: 'Business' }));
    expect(result.result).toBe(false);
    expect(result.error).toBe('tier');
  });

  it('should deny access when no license (null)', () => {
    const result = canUseEnterpriseFeature(null);
    expect(result.result).toBe(false);
    expect(result.error).toBe('tier');
  });

  it('should deny access when Enterprise license is expired', () => {
    const result = canUseEnterpriseFeature(
      makeLicense({ tier: 'Enterprise', expired: true }),
    );
    expect(result.result).toBe(false);
    expect(result.error).toBe('expired');
  });

  // Additive feature flags: a non-Enterprise tier can be granted a single enterprise
  // capability via the license `features` array, which must ungate just that feature.
  it('should allow a requested feature granted on a Business tier via a flag', () => {
    const result = canUseEnterpriseFeature(
      makeLicense({ tier: 'Business', features: [LicenseFeature.DevicePosture] }),
      LicenseFeature.DevicePosture,
    );
    expect(result.result).toBe(true);
    expect(result.error).toBeNull();
  });

  it('should deny a feature that was not granted, even when another flag is present', () => {
    const result = canUseEnterpriseFeature(
      makeLicense({ tier: 'Business', features: [LicenseFeature.DevicePosture] }),
      LicenseFeature.ServiceLocations,
    );
    expect(result.result).toBe(false);
    expect(result.error).toBe('tier');
  });

  it('should allow a requested feature on an Enterprise tier (folded into features)', () => {
    const result = canUseEnterpriseFeature(
      makeLicense({ tier: 'Enterprise', features: [LicenseFeature.AclAllowedIps] }),
      LicenseFeature.AclAllowedIps,
    );
    expect(result.result).toBe(true);
    expect(result.error).toBeNull();
  });

  it('should still deny a granted feature when the license is expired', () => {
    const result = canUseEnterpriseFeature(
      makeLicense({
        tier: 'Business',
        expired: true,
        features: [LicenseFeature.DevicePosture],
      }),
      LicenseFeature.DevicePosture,
    );
    expect(result.result).toBe(false);
    expect(result.error).toBe('expired');
  });

  // Regression: the backend clears `features` to `[]` for an expired license while keeping
  // `expired: true`. With the grant checked before expiry, an expired Enterprise license
  // requesting a feature wrongly reported 'tier' (upgrade) instead of 'expired'.
  it('should report expired (not tier) for an expired Enterprise license with cleared features', () => {
    const result = canUseEnterpriseFeature(
      makeLicense({ tier: 'Enterprise', expired: true, features: [] }),
      LicenseFeature.DevicePosture,
    );
    expect(result.result).toBe(false);
    expect(result.error).toBe('expired');
  });

  it('should fall back to the strict Enterprise tier gate when no feature is requested', () => {
    const result = canUseEnterpriseFeature(
      makeLicense({ tier: 'Business', features: [LicenseFeature.DevicePosture] }),
    );
    expect(result.result).toBe(false);
    expect(result.error).toBe('tier');
  });
});

describe('canUseServiceLocations', () => {
  // Regression: gating on a strict Enterprise-tier check ignored the additive flag, so a
  // Business license carrying ServiceLocations was wrongly blocked.
  it('should allow a Business license that carries the ServiceLocations flag', () => {
    expect(
      canUseServiceLocations(
        makeLicense({ tier: 'Business', features: [LicenseFeature.ServiceLocations] }),
      ),
    ).toBe(true);
  });

  it('should deny a Business license without the flag', () => {
    expect(canUseServiceLocations(makeLicense({ tier: 'Business', features: [] }))).toBe(
      false,
    );
  });

  it('should deny a Business license that has other flags but not ServiceLocations', () => {
    expect(
      canUseServiceLocations(
        makeLicense({ tier: 'Business', features: [LicenseFeature.DevicePosture] }),
      ),
    ).toBe(false);
  });

  // The backend folds the tier baseline into `features`, so a real Enterprise license always
  // carries every feature; model it that way.
  it('should allow an Enterprise license (every feature folded into features)', () => {
    expect(
      canUseServiceLocations(
        makeLicense({ tier: 'Enterprise', features: Object.values(LicenseFeature) }),
      ),
    ).toBe(true);
  });

  it('should deny an expired license even with the flag granted', () => {
    expect(
      canUseServiceLocations(
        makeLicense({
          tier: 'Business',
          expired: true,
          features: [LicenseFeature.ServiceLocations],
        }),
      ),
    ).toBe(false);
  });

  it('should deny when there is no license', () => {
    expect(canUseServiceLocations(null)).toBe(false);
  });
});

describe('getAdditiveFeatures', () => {
  it('should return empty array for Enterprise license (all features are tier-included)', () => {
    const license = makeLicense({
      tier: 'Enterprise',
      features: Object.values(LicenseFeature),
    });
    expect(getAdditiveFeatures(license)).toEqual([]);
  });

  it('should return explicit flags for Business license with granted features', () => {
    const license = makeLicense({
      tier: 'Business',
      features: [LicenseFeature.DevicePosture],
    });
    expect(getAdditiveFeatures(license)).toEqual([LicenseFeature.DevicePosture]);
  });

  it('should return empty array for Business license with no features', () => {
    const license = makeLicense({ tier: 'Business', features: [] });
    expect(getAdditiveFeatures(license)).toEqual([]);
  });

  it('should return multiple additive flags for Business license', () => {
    const license = makeLicense({
      tier: 'Business',
      features: [LicenseFeature.DevicePosture, LicenseFeature.ComponentHa],
    });
    expect(getAdditiveFeatures(license)).toEqual([
      LicenseFeature.DevicePosture,
      LicenseFeature.ComponentHa,
    ]);
  });
});

describe('canUseEnterpriseFeature per-feature additive grants', () => {
  for (const feature of Object.values(LicenseFeature)) {
    it(`unlocks only ${feature} when granted alone on a Business tier`, () => {
      const license = makeLicense({ tier: 'Business', features: [feature] });
      expect(canUseEnterpriseFeature(license, feature).result).toBe(true);
      for (const other of Object.values(LicenseFeature)) {
        if (other === feature) continue;
        expect(canUseEnterpriseFeature(license, other).result).toBe(false);
      }
    });
  }
});

describe('licenseActionCheck', () => {
  beforeEach(() => {
    vi.mocked(openModal).mockClear();
  });

  it('runs the success callback and opens no modal when the check passes', () => {
    const success = vi.fn();
    licenseActionCheck({ result: true, error: null, tierCheck: 'Enterprise' }, success);
    expect(success).toHaveBeenCalledOnce();
    expect(openModal).not.toHaveBeenCalled();
  });

  it('opens the business upgrade modal on a Business tier failure', () => {
    const success = vi.fn();
    licenseActionCheck({ result: false, error: 'tier', tierCheck: 'Business' }, success);
    expect(success).not.toHaveBeenCalled();
    expect(openModal).toHaveBeenCalledWith(ModalName.UpgradeBusiness);
  });

  it('opens the enterprise upgrade modal on an Enterprise tier failure', () => {
    licenseActionCheck(
      { result: false, error: 'tier', tierCheck: 'Enterprise' },
      vi.fn(),
    );
    expect(openModal).toHaveBeenCalledWith(ModalName.UpgradeEnterprise);
  });

  it('opens the expired modal carrying the failing tier', () => {
    licenseActionCheck(
      { result: false, error: 'expired', tierCheck: 'Enterprise' },
      vi.fn(),
    );
    expect(openModal).toHaveBeenCalledWith(ModalName.LicenseExpired, {
      licenseTier: 'Enterprise',
    });
  });
});

describe('isNavItemLocked', () => {
  it('never locks an entry without a tier requirement', () => {
    expect(isNavItemLocked(null, undefined)).toBe(false);
    expect(isNavItemLocked(makeLicense(), undefined)).toBe(false);
  });

  it('locks a Business entry only without a valid base license', () => {
    expect(isNavItemLocked(makeLicense({ tier: 'Business' }), LicenseTier.Business)).toBe(
      false,
    );
    expect(isNavItemLocked(null, LicenseTier.Business)).toBe(true);
    expect(isNavItemLocked(makeLicense({ expired: true }), LicenseTier.Business)).toBe(
      true,
    );
  });

  // Regression: an Enterprise nav entry must unlock on a lower tier carrying the additive flag.
  it('unlocks an Enterprise entry on a Business tier that carries the flag', () => {
    const license = makeLicense({
      tier: 'Business',
      features: [LicenseFeature.DevicePosture],
    });
    expect(
      isNavItemLocked(license, LicenseTier.Enterprise, LicenseFeature.DevicePosture),
    ).toBe(false);
  });

  it('locks an Enterprise entry on a Business tier missing the flag', () => {
    const license = makeLicense({ tier: 'Business', features: [] });
    expect(
      isNavItemLocked(license, LicenseTier.Enterprise, LicenseFeature.DevicePosture),
    ).toBe(true);
  });

  it('locks an Enterprise entry with no feature on anything below Enterprise tier', () => {
    const license = makeLicense({
      tier: 'Business',
      features: [LicenseFeature.DevicePosture],
    });
    expect(isNavItemLocked(license, LicenseTier.Enterprise)).toBe(true);
  });
});
