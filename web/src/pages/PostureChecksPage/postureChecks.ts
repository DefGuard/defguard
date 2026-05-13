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
  type PostureCheckRequirementValue,
  type PostureCheckVersionValues,
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
};

export type PostureCheckColumnFilterOptions = {
  windows: SelectionOption<PostureCheckFilterValue>[];
  macos: SelectionOption<PostureCheckFilterValue>[];
  linux: SelectionOption<PostureCheckFilterValue>[];
  ios: SelectionOption<PostureCheckFilterValue>[];
  android: SelectionOption<PostureCheckFilterValue>[];
  defguard: SelectionOption<PostureCheckFilterValue>[];
};

const requirementFilterDefinitions = {
  [PostureCheckRequirement.DiskEncryption]: {
    label: PostureCheckRequirement.DiskEncryption,
  },
  [PostureCheckRequirement.Antivirus]: {
    label: PostureCheckRequirement.Antivirus,
  },
  [PostureCheckRequirement.AdJoined]: {
    label: PostureCheckRequirement.AdJoined,
  },
  [PostureCheckRequirement.SecurityUpdates]: {
    label: PostureCheckRequirement.SecurityUpdates,
  },
  [PostureCheckRequirement.DeviceIntegrity]: {
    label: PostureCheckRequirement.DeviceIntegrity,
  },
  [PostureCheckRequirement.PrereleaseAllowed]: {
    label: PostureCheckRequirement.PrereleaseAllowed,
  },
} as const satisfies Record<PostureCheckRequirementValue, PostureCheckFilterDefinition>;

export const getPostureCheckTableFilterMessages = (): TableFilterMessages => ({
  searchPlaceholder: m.controls_search(),
  clearButton: m.controls_reset(),
  applyButton: m.controls_submit(),
  emptyState: m.search_empty_common_title(),
});

const toSelectionOptions = (
  values: readonly PostureCheckFilterValue[],
  getLabel: (value: string) => string,
): SelectionOption<PostureCheckFilterValue>[] =>
  values.map((value) => ({
    id: value,
    label: getLabel(value),
  }));

const toRequirementSelectionOptions = (
  values: readonly PostureCheckRequirementValue[],
): SelectionOption<PostureCheckFilterValue>[] =>
  values.map((value) => ({
    id: value,
    label: requirementFilterDefinitions[value].label,
  }));

export const getPostureCheckColumnFilterOptions = (
  versionValues: PostureCheckVersionValues,
): PostureCheckColumnFilterOptions => ({
  windows: [
    ...toSelectionOptions(versionValues.windows, (value) => value),
    ...toRequirementSelectionOptions([
      PostureCheckRequirement.DiskEncryption,
      PostureCheckRequirement.Antivirus,
      PostureCheckRequirement.AdJoined,
      PostureCheckRequirement.SecurityUpdates,
    ]),
  ],
  macos: [
    ...toSelectionOptions(versionValues.macos, (value) => value),
    ...toRequirementSelectionOptions([
      PostureCheckRequirement.DiskEncryption,
      PostureCheckRequirement.DeviceIntegrity,
    ]),
  ],
  linux: [
    ...toSelectionOptions(versionValues.linux, (value) => `Kernel ${value}`),
    ...toRequirementSelectionOptions([PostureCheckRequirement.DiskEncryption]),
  ],
  ios: toSelectionOptions(versionValues.ios, (value) => `iOS ${value}+`),
  android: [
    ...toSelectionOptions(versionValues.android, (value) => `Android ${value}+`),
    ...toRequirementSelectionOptions([PostureCheckRequirement.DeviceIntegrity]),
  ],
  defguard: [
    ...toSelectionOptions(versionValues.defguard, (value) => `Defguard ${value}+`),
    ...toRequirementSelectionOptions([PostureCheckRequirement.PrereleaseAllowed]),
  ],
});

export const mapPostureCheckFilterValueToRequestValue = (
  value: PostureCheckFilterValue,
) => value;

export const isPostureCheckFilterValue = (
  value: string,
): value is PostureCheckFilterValue => value.length > 0;

const mapVersionFilterValue = (value: string | undefined | null) => value ?? undefined;

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

const getOsRuleParts = (
  rule: ApiDevicePostureOsRule | undefined,
): PostureCheckRuleParts => {
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
    posture.allow_prerelease_client && 'Pre-release allowed',
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
