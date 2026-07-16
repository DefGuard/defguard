import { describe, expect, it } from 'vitest';
import { buildAddPostureCheckRequest } from '../src/pages/AddPostureCheckWizardPage/payload';
import { PostureCheckOs } from '../src/pages/PostureChecksPage/types';

describe('add posture check payload', () => {
  const emptyOperatingSystemState = {
    [PostureCheckOs.Windows]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Macos]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Linux]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Ios]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Android]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
  };

  it('preserves Any version as null in the API request', () => {
    expect(
      buildAddPostureCheckRequest({
        allowPrereleaseClient: false,
        configuredOperatingSystems: [PostureCheckOs.Windows, PostureCheckOs.Linux],
        description: null,
        minimumDesktopClientVersion: null,
        minimumMobileClientVersion: null,
        name: 'Any version policy',
        operatingSystemState: {
          ...emptyOperatingSystemState,
          [PostureCheckOs.Windows]: {
            ...emptyOperatingSystemState[PostureCheckOs.Windows],
            conditions: ['disk-encryption'],
          },
        },
      }),
    ).toEqual({
      name: 'Any version policy',
      description: null,
      min_desktop_client_version: null,
      min_mobile_client_version: null,
      allow_prerelease_client: false,
      os_rules: [
        {
          os_type: 'windows',
          min_os_version: null,
          disk_encryption_required: true,
          antivirus_required: null,
          ad_domain_joined_required: null,
          windows_security_update_max_age: null,
        },
        {
          os_type: 'linux',
          min_kernel_version: null,
          disk_encryption_required: null,
        },
      ],
    });
  });

  it('sends desktop and mobile client-version requirements independently', () => {
    expect(
      buildAddPostureCheckRequest({
        allowPrereleaseClient: true,
        configuredOperatingSystems: [],
        description: 'Split client requirements',
        minimumDesktopClientVersion: '2.1',
        minimumMobileClientVersion: '1.7.0',
        name: 'Client version policy',
        operatingSystemState: emptyOperatingSystemState,
      }),
    ).toEqual({
      name: 'Client version policy',
      description: 'Split client requirements',
      min_desktop_client_version: '2.1',
      min_mobile_client_version: '1.7.0',
      allow_prerelease_client: true,
      os_rules: [],
    });
  });
});
