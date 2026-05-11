import { m } from '../../paraglide/messages';
import type { ApiDevicePosture, ApiDevicePostureOsRule } from '../../shared/api/types';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import type { TableFilterMessages } from '../../shared/defguard-ui/components/table/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import {
  type PostureCheckFilterValue,
  PostureCheckOs,
  type PostureCheckOsValue,
  PostureCheckRequirement,
  postureCheckVersionValues,
} from './types';

export type PostureCheckRow = {
  id: number;
  name: string;
  windows: string;
  windowsFilters: PostureCheckFilterValue[];
  macos: string;
  macosFilters: PostureCheckFilterValue[];
  linux: string;
  linuxFilters: PostureCheckFilterValue[];
  ios: string;
  iosFilters: PostureCheckFilterValue[];
  android: string;
  androidFilters: PostureCheckFilterValue[];
  defguard: string;
  defguardFilters: PostureCheckFilterValue[];
};

const emptyRequirement = '-';

type PostureCheckFilterDefinition = {
  label: string;
  requestValue: string;
};

const postureCheckFilterDefinitions = {
  'Windows 10': { label: 'Windows 10', requestValue: 'Windows 10' },
  'Windows 11': { label: 'Windows 11', requestValue: 'Windows 11' },
  'macOS 12 Monterey': {
    label: 'macOS 12 Monterey',
    requestValue: 'macOS 12 Monterey',
  },
  'macOS 13 Ventura': {
    label: 'macOS 13 Ventura',
    requestValue: 'macOS 13 Ventura',
  },
  'macOS 14 Sonoma': {
    label: 'macOS 14 Sonoma',
    requestValue: 'macOS 14 Sonoma',
  },
  'macOS 15 Sequoia': {
    label: 'macOS 15 Sequoia',
    requestValue: 'macOS 15 Sequoia',
  },
  '5.x': { label: 'Kernel 5.x', requestValue: '5.x' },
  '6.x': { label: 'Kernel 6.x', requestValue: '6.x' },
  '17': { label: 'iOS 17+', requestValue: '17' },
  '18': { label: 'iOS 18+', requestValue: '18' },
  '13': { label: 'Android 13+', requestValue: '13' },
  '14': { label: 'Android 14+', requestValue: '14' },
  '15': { label: 'Android 15+', requestValue: '15' },
  '16': { label: 'Android 16+', requestValue: '16' },
  '1.6': { label: 'Defguard 1.6+', requestValue: '1.6' },
  '2.0': { label: 'Defguard 2.0+', requestValue: '2.0' },
  [PostureCheckRequirement.DiskEncryption]: {
    label: PostureCheckRequirement.DiskEncryption,
    requestValue: PostureCheckRequirement.DiskEncryption,
  },
  [PostureCheckRequirement.Antivirus]: {
    label: PostureCheckRequirement.Antivirus,
    requestValue: PostureCheckRequirement.Antivirus,
  },
  [PostureCheckRequirement.AdJoined]: {
    label: PostureCheckRequirement.AdJoined,
    requestValue: PostureCheckRequirement.AdJoined,
  },
  [PostureCheckRequirement.SecurityUpdates]: {
    label: PostureCheckRequirement.SecurityUpdates,
    requestValue: PostureCheckRequirement.SecurityUpdates,
  },
  [PostureCheckRequirement.DeviceIntegrity]: {
    label: PostureCheckRequirement.DeviceIntegrity,
    requestValue: PostureCheckRequirement.DeviceIntegrity,
  },
  [PostureCheckRequirement.PrereleaseAllowed]: {
    label: PostureCheckRequirement.PrereleaseAllowed,
    requestValue: PostureCheckRequirement.PrereleaseAllowed,
  },
} as const satisfies Record<PostureCheckFilterValue, PostureCheckFilterDefinition>;

export const postureCheckFilterOptions = {
  windows: [
    ...postureCheckVersionValues.windows,
    PostureCheckRequirement.DiskEncryption,
    PostureCheckRequirement.Antivirus,
    PostureCheckRequirement.AdJoined,
    PostureCheckRequirement.SecurityUpdates,
  ],
  macos: [
    ...postureCheckVersionValues.macos,
    PostureCheckRequirement.DiskEncryption,
    PostureCheckRequirement.DeviceIntegrity,
  ],
  linux: [...postureCheckVersionValues.linux, PostureCheckRequirement.DiskEncryption],
  ios: postureCheckVersionValues.ios,
  android: [
    ...postureCheckVersionValues.android,
    PostureCheckRequirement.DeviceIntegrity,
  ],
  defguard: [
    ...postureCheckVersionValues.defguard,
    PostureCheckRequirement.PrereleaseAllowed,
  ],
} as const;

export const getPostureCheckTableFilterMessages = (): TableFilterMessages => ({
  searchPlaceholder: m.controls_search(),
  clearButton: m.controls_reset(),
  applyButton: m.controls_submit(),
  emptyState: m.search_empty_common_title(),
});

const toSelectionOptions = <T extends PostureCheckFilterValue>(
  values: readonly T[],
): SelectionOption<T>[] =>
  values.map((value) => ({
    id: value,
    label: postureCheckFilterDefinitions[value].label,
  }));

export const postureCheckColumnFilterOptions = {
  windows: toSelectionOptions(postureCheckFilterOptions.windows),
  macos: toSelectionOptions(postureCheckFilterOptions.macos),
  linux: toSelectionOptions(postureCheckFilterOptions.linux),
  ios: toSelectionOptions(postureCheckFilterOptions.ios),
  android: toSelectionOptions(postureCheckFilterOptions.android),
  defguard: toSelectionOptions(postureCheckFilterOptions.defguard),
};

export const mapPostureCheckFilterValueToRequestValue = (
  value: PostureCheckFilterValue,
) => postureCheckFilterDefinitions[value].requestValue;

export const isPostureCheckFilterValue = (
  value: string,
): value is PostureCheckFilterValue => value in postureCheckFilterDefinitions;

const mapVersionFilterValue = (value: string | undefined | null) =>
  value && isPostureCheckFilterValue(value) ? value : undefined;

const joinRequirementParts = (parts: Array<string | null | undefined | false>) => {
  const filteredParts = parts.filter((part): part is string => Boolean(part));

  return filteredParts.length ? filteredParts.join(', ') : emptyRequirement;
};

const joinFilters = (parts: Array<PostureCheckFilterValue | null | undefined | false>) =>
  parts.filter((part): part is PostureCheckFilterValue => Boolean(part));

type PostureCheckRuleParts = {
  summaryParts: Array<string | null | undefined | false>;
  filterParts: Array<PostureCheckFilterValue | null | undefined | false>;
};

const emptyPostureCheckRuleParts: PostureCheckRuleParts = {
  summaryParts: [],
  filterParts: [],
};

const getOsRuleParts = (rule: ApiDevicePostureOsRule | undefined): PostureCheckRuleParts => {
  if (!isPresent(rule)) {
    return emptyPostureCheckRuleParts;
  }

  switch (rule.os_type) {
    case PostureCheckOs.Windows:
      return {
        summaryParts: [
          rule.min_os_version,
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
          rule.antivirus_required && PostureCheckRequirement.Antivirus,
          rule.ad_domain_joined_required && PostureCheckRequirement.AdJoined,
          rule.windows_security_update_current && PostureCheckRequirement.SecurityUpdates,
        ],
        filterParts: [
          mapVersionFilterValue(rule.min_os_version),
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
          rule.antivirus_required && PostureCheckRequirement.Antivirus,
          rule.ad_domain_joined_required && PostureCheckRequirement.AdJoined,
          rule.windows_security_update_current && PostureCheckRequirement.SecurityUpdates,
        ],
      };
    case PostureCheckOs.Macos:
      return {
        summaryParts: [
          rule.min_os_version,
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
          rule.device_integrity_required && PostureCheckRequirement.DeviceIntegrity,
        ],
        filterParts: [
          mapVersionFilterValue(rule.min_os_version),
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
          rule.device_integrity_required && PostureCheckRequirement.DeviceIntegrity,
        ],
      };
    case PostureCheckOs.Linux:
      return {
        summaryParts: [
          rule.min_kernel_version ? `Kernel ${rule.min_kernel_version}` : null,
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
        ],
        filterParts: [
          mapVersionFilterValue(rule.min_kernel_version),
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
        ],
      };
    case PostureCheckOs.Ios:
      return {
        summaryParts: [rule.min_os_version ? `iOS ${rule.min_os_version}+` : null],
        filterParts: [mapVersionFilterValue(rule.min_os_version)],
      };
    case PostureCheckOs.Android:
      return {
        summaryParts: [
          rule.min_os_version ? `Android ${rule.min_os_version}+` : null,
          rule.device_integrity_required && PostureCheckRequirement.DeviceIntegrity,
        ],
        filterParts: [
          mapVersionFilterValue(rule.min_os_version),
          rule.device_integrity_required && PostureCheckRequirement.DeviceIntegrity,
        ],
      };
    default:
      return emptyPostureCheckRuleParts;
  }
};

const getOsRuleSummary = (rule: ApiDevicePostureOsRule | undefined) =>
  joinRequirementParts(getOsRuleParts(rule).summaryParts);

const getOsRuleFilters = (rule: ApiDevicePostureOsRule | undefined) =>
  joinFilters(getOsRuleParts(rule).filterParts);

const getDevicePostureRule = (
  posture: ApiDevicePosture,
  osType: PostureCheckOsValue,
): ApiDevicePostureOsRule | undefined =>
  posture.os_rules.find((rule) => rule.os_type === osType);

export const mapApiDevicePostureToRow = (posture: ApiDevicePosture): PostureCheckRow => ({
  id: posture.id,
  name: posture.name,
  windows: getOsRuleSummary(getDevicePostureRule(posture, PostureCheckOs.Windows)),
  windowsFilters: getOsRuleFilters(getDevicePostureRule(posture, PostureCheckOs.Windows)),
  macos: getOsRuleSummary(getDevicePostureRule(posture, PostureCheckOs.Macos)),
  macosFilters: getOsRuleFilters(getDevicePostureRule(posture, PostureCheckOs.Macos)),
  linux: getOsRuleSummary(getDevicePostureRule(posture, PostureCheckOs.Linux)),
  linuxFilters: getOsRuleFilters(getDevicePostureRule(posture, PostureCheckOs.Linux)),
  ios: getOsRuleSummary(getDevicePostureRule(posture, PostureCheckOs.Ios)),
  iosFilters: getOsRuleFilters(getDevicePostureRule(posture, PostureCheckOs.Ios)),
  android: getOsRuleSummary(getDevicePostureRule(posture, PostureCheckOs.Android)),
  androidFilters: getOsRuleFilters(getDevicePostureRule(posture, PostureCheckOs.Android)),
  defguard: joinRequirementParts([
    posture.min_client_version ? `Defguard ${posture.min_client_version}+` : null,
    posture.allow_prerelease_client && 'Prerelease allowed',
  ]),
  defguardFilters: joinFilters([
    mapVersionFilterValue(posture.min_client_version),
    posture.allow_prerelease_client && PostureCheckRequirement.PrereleaseAllowed,
  ]),
});

export const getPostureCheckOsLabel = (value: PostureCheckOsValue) => {
  switch (value) {
    case PostureCheckOs.Windows:
      return 'Windows';
    case PostureCheckOs.Macos:
      return 'macOS';
    case PostureCheckOs.Linux:
      return 'Linux';
    case PostureCheckOs.Ios:
      return 'iOS';
    default:
      return 'Android';
  }
};

export const filterPostureChecks = (rows: PostureCheckRow[], search: string) => {
  const query = search.trim().toLowerCase();

  if (!query.length) {
    return rows;
  }

  return rows.filter((row) => {
    const searchableValues = [
      row.name,
      row.windows,
      row.macos,
      row.linux,
      row.ios,
      row.android,
      row.defguard,
    ];

    return searchableValues.some((value) => value.toLowerCase().includes(query));
  });
};
