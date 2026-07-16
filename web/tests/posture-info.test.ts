import { describe, expect, it } from 'vitest';
import type { ApiDevicePosture } from '../src/shared/api/types';
import { buildOsSections } from '../src/shared/utils/postureInfo';

describe('posture check drawer info', () => {
  it('shows Any version for configured null-version rules', () => {
    const postureCheck: ApiDevicePosture = {
      id: 1,
      name: 'Any version policy',
      description: null,
      min_desktop_client_version: null,
      min_mobile_client_version: null,
      allow_prerelease_client: false,
      locations: [],
      os_rules: [
        {
          os_type: 'windows',
          min_os_version: null,
          disk_encryption_required: true,
          antivirus_required: false,
          ad_domain_joined_required: false,
          windows_security_update_max_age: null,
        },
      ],
    };

    expect(buildOsSections(postureCheck)).toEqual([
      expect.objectContaining({
        name: 'Windows',
        rows: [
          { label: 'Version', value: 'Any version' },
          { label: 'Other', value: ['Disk encryption enabled'] },
        ],
      }),
      expect.objectContaining({
        name: 'Defguard',
        rows: [
          { label: 'Desktop', value: 'Any version' },
          { label: 'Mobile app', value: 'Any version' },
        ],
      }),
    ]);
  });
});
