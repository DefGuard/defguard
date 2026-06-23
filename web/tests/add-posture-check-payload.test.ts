import { describe, expect, it } from 'vitest';
import { buildAddPostureCheckRequest } from '../src/pages/AddPostureCheckWizardPage/payload';
import { PostureCheckOs } from '../src/pages/PostureChecksPage/types';

describe('add posture check payload', () => {
  it('preserves Any version as null in the API request', () => {
    expect(
      buildAddPostureCheckRequest({
        allowPrereleaseClient: false,
        configuredOperatingSystems: [PostureCheckOs.Windows, PostureCheckOs.Linux],
        description: null,
        minimumClientVersion: null,
        name: 'Any version policy',
        operatingSystemState: {
          [PostureCheckOs.Windows]: {
            conditions: ['disk-encryption'],
            securityUpdateMaxAge: null,
            version: null,
          },
          [PostureCheckOs.Macos]: {
            conditions: [],
            securityUpdateMaxAge: null,
            version: null,
          },
          [PostureCheckOs.Linux]: {
            conditions: [],
            securityUpdateMaxAge: null,
            version: null,
          },
          [PostureCheckOs.Ios]: {
            conditions: [],
            securityUpdateMaxAge: null,
            version: null,
          },
          [PostureCheckOs.Android]: {
            conditions: [],
            securityUpdateMaxAge: null,
            version: null,
          },
        },
      }),
    ).toEqual({
      name: 'Any version policy',
      description: null,
      min_client_version: null,
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
});
