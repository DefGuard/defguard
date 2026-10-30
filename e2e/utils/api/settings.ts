import { APIRequestContext, expect } from '@playwright/test';

import { defaultUserAdmin, testsConfig } from '../../config';
import { apiLogin } from './users';

export type LicenseTier = 'Business' | 'Enterprise';

type LicenseInfoResponse = {
  license_info: {
    tier: LicenseTier;
    expired: boolean;
    limits_exceeded: boolean;
  } | null;
};

export const expectLicenseTier = async (
  api: APIRequestContext,
  tier: LicenseTier | null,
): Promise<void> => {
  const response = await api.get(`${testsConfig.CORE_BASE_URL}/enterprise_info`);
  expect(response.status()).toBe(200);
  const { license_info }: LicenseInfoResponse = await response.json();

  if (tier === null) {
    expect(license_info).toBeNull();
    return;
  }

  expect(license_info).toEqual(
    expect.objectContaining({ tier, expired: false, limits_exceeded: false }),
  );
};

export const setLicense = async (
  api: APIRequestContext,
  license: string | null,
  tier: LicenseTier | null,
): Promise<void> => {
  expect(await apiLogin(api, defaultUserAdmin.username, defaultUserAdmin.password)).toBe(
    200,
  );
  const response = await api.patch(`${testsConfig.CORE_BASE_URL}/settings`, {
    data: { license: license?.trim() ?? '' },
  });
  expect(response.ok()).toBe(true);
  await expectLicenseTier(api, tier);
};
