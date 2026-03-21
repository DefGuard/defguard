import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import { describe, expect, it } from 'vitest';
import type { LicenseInfo } from '../src/shared/api/types';
import {
  canUseBusinessFeature,
  canUseEnterpriseFeature,
  getLicenseState,
} from '../src/shared/utils/license';

dayjs.extend(utc);

const makeLicense = (overrides: Partial<LicenseInfo> = {}): LicenseInfo => ({
  subscription: false,
  valid_until: null,
  expired: false,
  limits_exceeded: false,
  tier: 'Business',
  limits: null,
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
    const result = canUseEnterpriseFeature(makeLicense({ tier: 'Enterprise', expired: true }));
    expect(result.result).toBe(false);
    expect(result.error).toBe('expired');
  });
});
