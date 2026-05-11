import { describe, expect, it } from 'vitest';
import {
  filterPostureChecks,
  getPostureCheckOsLabel,
  mapApiDevicePostureToRow,
  mapPostureCheckFilterValueToRequestValue,
  postureCheckColumnFilterOptions,
  type PostureCheckRow,
} from '../src/pages/PostureChecksPage/postureChecks';
import { PostureCheckOs, PostureCheckRequirement } from '../src/pages/PostureChecksPage/types';
import type { ApiDevicePosture } from '../src/shared/api/types';

describe('posture checks page helpers', () => {
  it('maps fetched posture-check policies into compact table rows', () => {
    const postureCheck: ApiDevicePosture = {
      id: 1,
      name: 'First posture check',
      description: 'Example posture check',
      min_client_version: '1.6',
      allow_prerelease_client: true,
      locations: [2],
      os_rules: [
        {
          os_type: 'windows',
          min_os_version: 'Windows 11',
          disk_encryption_required: true,
          antivirus_required: true,
          ad_domain_joined_required: false,
          windows_security_update_current: false,
        },
        {
          os_type: 'linux',
          min_kernel_version: '6.x',
          disk_encryption_required: true,
        },
        {
          os_type: 'ios',
          min_os_version: '17',
        },
        {
          os_type: 'android',
          min_os_version: '15',
          device_integrity_required: true,
        },
      ],
    };

    expect(mapApiDevicePostureToRow(postureCheck)).toEqual({
      id: 1,
      name: 'First posture check',
      windows: 'Windows 11, Disk encryption, Antivirus',
      windowsFilters: ['Windows 11', PostureCheckRequirement.DiskEncryption, 'Antivirus'],
      macos: '-',
      macosFilters: [],
      linux: 'Kernel 6.x, Disk encryption',
      linuxFilters: ['6.x', PostureCheckRequirement.DiskEncryption],
      ios: 'iOS 17+',
      iosFilters: ['17'],
      android: 'Android 15+, Device integrity',
      androidFilters: ['15', PostureCheckRequirement.DeviceIntegrity],
      defguard: 'Defguard 1.6+, Prerelease allowed',
      defguardFilters: ['1.6', PostureCheckRequirement.PrereleaseAllowed],
    });
  });

  it('should format posture-check labels consistently with the wizard review step', () => {
    expect(getPostureCheckOsLabel(PostureCheckOs.Windows)).toBe('Windows');
    expect(getPostureCheckOsLabel(PostureCheckOs.Macos)).toBe('macOS');
    expect(getPostureCheckOsLabel(PostureCheckOs.Ios)).toBe('iOS');
  });

  it('filters posture checks using API-derived requirement summaries', () => {
    const rows: PostureCheckRow[] = [
      {
        id: 99,
        name: 'First posture check',
        windows: 'Windows 11, Disk encryption, Antivirus',
        windowsFilters: ['Windows 11', PostureCheckRequirement.DiskEncryption, 'Antivirus'],
        macos: '-',
        macosFilters: [],
        linux: 'Kernel 6.x, Disk encryption',
        linuxFilters: ['6.x', PostureCheckRequirement.DiskEncryption],
        ios: 'iOS 17+',
        iosFilters: ['17'],
        android: 'Android 15+, Device integrity',
        androidFilters: ['15', PostureCheckRequirement.DeviceIntegrity],
        defguard: 'Defguard 1.6+, Prerelease allowed',
        defguardFilters: ['1.6', PostureCheckRequirement.PrereleaseAllowed],
      },
      {
        id: 100,
        name: 'Second posture check',
        windows: '-',
        windowsFilters: [],
        macos: 'macOS 15 Sequoia, Device integrity',
        macosFilters: ['macOS 15 Sequoia', PostureCheckRequirement.DeviceIntegrity],
        linux: '-',
        linuxFilters: [],
        ios: '-',
        iosFilters: [],
        android: '-',
        androidFilters: [],
        defguard: '-',
        defguardFilters: [],
      },
    ];

    expect(filterPostureChecks(rows, 'second posture')).toEqual([rows[1]]);
    expect(filterPostureChecks(rows, 'android 15+')).toEqual([rows[0]]);
    expect(filterPostureChecks(rows, 'prerelease allowed')).toEqual([rows[0]]);
    expect(filterPostureChecks(rows, '')).toEqual(rows);
    expect(filterPostureChecks(rows, 'not-present')).toEqual([]);
  });

  it('exposes predefined filter buckets for posture requirement columns', () => {
    expect(postureCheckColumnFilterOptions.windows.map((option) => option.id)).toEqual([
      'Windows 10',
      'Windows 11',
      PostureCheckRequirement.DiskEncryption,
      PostureCheckRequirement.Antivirus,
      PostureCheckRequirement.AdJoined,
      PostureCheckRequirement.SecurityUpdates,
    ]);
    expect(postureCheckColumnFilterOptions.defguard.map((option) => option.id)).toEqual([
      '1.6',
      '2.0',
      PostureCheckRequirement.PrereleaseAllowed,
    ]);
  });

  it('maps typed filter values to backend request values', () => {
    expect(mapPostureCheckFilterValueToRequestValue('Windows 11')).toBe('Windows 11');
    expect(mapPostureCheckFilterValueToRequestValue('6.x')).toBe('6.x');
    expect(mapPostureCheckFilterValueToRequestValue('1.6')).toBe('1.6');
    expect(mapPostureCheckFilterValueToRequestValue(PostureCheckRequirement.PrereleaseAllowed)).toBe(
      'Prerelease allowed',
    );
  });
});
