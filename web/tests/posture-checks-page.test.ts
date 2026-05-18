import { describe, expect, it } from 'vitest';
import {
  getInitialEditPostureCheckFormValues,
  normalizeEditPostureCheckFormValues,
} from '../src/pages/EditPostureCheckPage/form';
import {
  filterPostureChecks,
  getPostureCheckColumnFilterOptions,
  getPostureCheckOsLabel,
  mapApiDevicePostureToRow,
  mapPostureCheckFilterValueToRequestValue,
  type PostureCheckRow,
} from '../src/pages/PostureChecksPage/postureChecks';
import { isPostureChecksListPath } from '../src/pages/PostureChecksPage/route';
import {
  getPostureCheckVersionValues,
  PostureCheckOs,
  PostureCheckRequirement,
} from '../src/pages/PostureChecksPage/types';
import type {
  ApiDevicePosture,
  DevicePostureVersionMetadata,
} from '../src/shared/api/types';

const makeVersionMetadata = (): DevicePostureVersionMetadata => ({
  os_versions: {
    windows: [10, 11],
    macos: [13, 14, 15, 26],
    ios: [17, 18, 26],
    android: [13, 14, 15, 16],
  },
  linux_kernel_versions: [5, 6, 7],
  client_versions: ['1.6', '2.0'],
});

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
          min_os_version: 11,
          disk_encryption_required: true,
          antivirus_required: true,
          ad_domain_joined_required: false,
          windows_security_update_current: false,
        },
        {
          os_type: 'linux',
          min_kernel_version: 6,
          disk_encryption_required: true,
        },
        {
          os_type: 'ios',
          min_os_version: 17,
        },
        {
          os_type: 'android',
          min_os_version: 15,
          device_integrity_required: true,
        },
      ],
    };

    expect(mapApiDevicePostureToRow(postureCheck)).toEqual({
      id: 1,
      name: 'First posture check',
      windows: '11, Disk encryption, Antivirus',
      windowsFilters: [11, PostureCheckRequirement.DiskEncryption, 'Antivirus'],
      macos: '-',
      macosFilters: [],
      linux: 'Kernel 6, Disk encryption',
      linuxFilters: [6, PostureCheckRequirement.DiskEncryption],
      ios: 'iOS 17+',
      iosFilters: [17],
      android: 'Android 15+, Device integrity',
      androidFilters: [15, PostureCheckRequirement.DeviceIntegrity],
      defguard: 'Defguard 1.6+, Pre-release allowed',
      defguardFilters: ['1.6', PostureCheckRequirement.PrereleaseAllowed],
    });
  });

  it('uses empty placeholders when an OS rule is missing or has no requirements', () => {
    const postureCheck: ApiDevicePosture = {
      id: 2,
      name: 'Second posture check',
      description: 'Example posture check',
      min_client_version: null,
      allow_prerelease_client: false,
      locations: [],
      os_rules: [
        {
          os_type: 'windows',
          min_os_version: null,
          disk_encryption_required: false,
          antivirus_required: false,
          ad_domain_joined_required: false,
          windows_security_update_current: false,
        },
      ],
    };

    expect(mapApiDevicePostureToRow(postureCheck)).toEqual({
      id: 2,
      name: 'Second posture check',
      windows: '-',
      windowsFilters: [],
      macos: '-',
      macosFilters: [],
      linux: '-',
      linuxFilters: [],
      ios: '-',
      iosFilters: [],
      android: '-',
      androidFilters: [],
      defguard: '-',
      defguardFilters: [],
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
        windows: '11, Disk encryption, Antivirus',
        windowsFilters: [11, PostureCheckRequirement.DiskEncryption, 'Antivirus'],
        macos: '-',
        macosFilters: [],
        linux: 'Kernel 6, Disk encryption',
        linuxFilters: [6, PostureCheckRequirement.DiskEncryption],
        ios: 'iOS 17+',
        iosFilters: [17],
        android: 'Android 15+, Device integrity',
        androidFilters: [15, PostureCheckRequirement.DeviceIntegrity],
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

  it('maps posture version metadata into UI version values, including Linux kernel versions', () => {
    const metadata: DevicePostureVersionMetadata = {
      os_versions: {
        windows: [11],
        macos: [15],
        ios: [18],
        android: [15],
      },
      linux_kernel_versions: [6, 7],
      client_versions: ['1.6'],
    };

    expect(getPostureCheckVersionValues(metadata)).toEqual({
      windows: [11],
      macos: [15],
      linux: [6, 7],
      ios: [18],
      android: [15],
      defguard: ['1.6'],
    });
  });

  it('exposes predefined filter buckets for posture requirement columns', () => {
    const postureCheckColumnFilterOptions = getPostureCheckColumnFilterOptions(
      getPostureCheckVersionValues(makeVersionMetadata()),
    );

    expect(postureCheckColumnFilterOptions.windows.map((option) => option.id)).toEqual([
      10,
      11,
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
    expect(mapPostureCheckFilterValueToRequestValue(11)).toBe('11');
    expect(mapPostureCheckFilterValueToRequestValue(6)).toBe('6');
    expect(mapPostureCheckFilterValueToRequestValue('1.6')).toBe('1.6');
    expect(
      mapPostureCheckFilterValueToRequestValue(PostureCheckRequirement.PrereleaseAllowed),
    ).toBe('Pre-release allowed');
  });

  it('maps an existing posture check into editable form state with assigned locations', () => {
    const postureCheck: ApiDevicePosture = {
      id: 5,
      name: 'Edit posture check',
      description: 'Existing policy',
      min_client_version: '2.0',
      allow_prerelease_client: true,
      locations: [9, 3],
      os_rules: [
        {
          os_type: 'windows',
          min_os_version: 11,
          disk_encryption_required: true,
          antivirus_required: false,
          ad_domain_joined_required: true,
          windows_security_update_current: true,
        },
        {
          os_type: 'android',
          min_os_version: 15,
          device_integrity_required: true,
        },
      ],
    };

    expect(
      normalizeEditPostureCheckFormValues(
        getInitialEditPostureCheckFormValues(
          postureCheck,
          getPostureCheckVersionValues(makeVersionMetadata()),
        ),
      ),
    ).toEqual({
      allowPrereleaseClient: true,
      configuredOperatingSystems: ['windows', 'android'],
      description: 'Existing policy',
      locations: [3, 9],
      minimumClientVersion: '2.0',
      name: 'Edit posture check',
      operatingSystemState: {
        windows: {
          conditions: ['active-directory', 'disk-encryption'],
          securityUpdates: true,
          version: 11,
        },
        macos: {
          conditions: [],
          securityUpdates: false,
          version: 26,
        },
        linux: {
          conditions: [],
          securityUpdates: false,
          version: 7,
        },
        ios: {
          conditions: [],
          securityUpdates: false,
          version: 26,
        },
        android: {
          conditions: ['device-integrity'],
          securityUpdates: false,
          version: 15,
        },
      },
    });
  });

  it('should render the list page only on the base posture checks route', () => {
    expect(isPostureChecksListPath('/acl/posture-checks')).toBe(true);
    expect(isPostureChecksListPath('/acl/posture-checks/5/edit')).toBe(false);
  });
});
