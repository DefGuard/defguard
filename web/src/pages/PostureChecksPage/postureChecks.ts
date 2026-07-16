import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type ApiDevicePosture,
  type ApiDevicePostureOsRule,
  LocationServiceMode,
  type NetworkLocation,
} from '../../shared/api/types';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import type { TableFilterMessages } from '../../shared/defguard-ui/components/table/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import type { OpenConfirmActionModal } from '../../shared/hooks/modalControls/types';
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
  locations: number[];
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
  defguardDesktop: string;
  defguardDesktopFilters: PostureCheckFilterValue[];
  defguardMobile: string;
  defguardMobileFilters: PostureCheckFilterValue[];
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
  defguard_desktop: SelectionOption<PostureCheckFilterValue>[];
  defguard_mobile: SelectionOption<PostureCheckFilterValue>[];
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
} as const satisfies Record<PostureCheckRequirementValue, PostureCheckFilterDefinition>;

export const getPostureCheckTableFilterMessages = (): TableFilterMessages => ({
  searchPlaceholder: m.controls_search(),
  clearButton: m.controls_reset(),
  applyButton: m.controls_submit(),
  emptyState: m.search_empty_common_title(),
});

const toSelectionOptions = <T extends PostureCheckFilterValue>(
  values: readonly T[],
  getLabel: (value: T) => string,
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
    ...toSelectionOptions(versionValues.windows, (value) => value.toString()),
    ...toRequirementSelectionOptions([
      PostureCheckRequirement.DiskEncryption,
      PostureCheckRequirement.Antivirus,
      PostureCheckRequirement.AdJoined,
      PostureCheckRequirement.SecurityUpdates,
    ]),
  ],
  macos: [
    ...toSelectionOptions(versionValues.macos, (value) => value.toString()),
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
  defguard_desktop: toSelectionOptions(
    versionValues.defguardDesktop,
    (value) => `${value}+`,
  ),
  defguard_mobile: toSelectionOptions(
    versionValues.defguardMobile,
    (value) => `${value}+`,
  ),
});

export const mapPostureCheckFilterValueToRequestValue = (
  value: PostureCheckFilterValue,
) => (typeof value === 'number' ? value.toString() : value);

export const isPostureCheckFilterValue = (
  value: string | number,
): value is PostureCheckFilterValue => typeof value === 'number' || value.length > 0;

const mapVersionFilterValue = (value: number | string | undefined | null) =>
  value ?? undefined;

const mapAnyVersionSummary = (
  value: number | undefined | null,
  getLabel: (value: number) => string,
) => (isPresent(value) ? getLabel(value) : m.posture_checks_version_any());

const joinRequirementParts = (parts: Array<string | null | undefined | false>) => {
  const filteredParts = parts.filter((part): part is string => Boolean(part));

  return filteredParts.length ? filteredParts.join(', ') : emptyRequirement;
};

const joinFilters = (parts: Array<PostureCheckFilterValue | null | undefined | false>) =>
  parts.filter(
    (part): part is PostureCheckFilterValue => part !== false && isPresent(part),
  );

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
          mapAnyVersionSummary(rule.min_os_version, (value) => value.toString()),
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
          rule.antivirus_required && PostureCheckRequirement.Antivirus,
          rule.ad_domain_joined_required && PostureCheckRequirement.AdJoined,
          rule.windows_security_update_max_age !== null &&
            PostureCheckRequirement.SecurityUpdates,
        ],
        filterParts: [
          mapVersionFilterValue(rule.min_os_version),
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
          rule.antivirus_required && PostureCheckRequirement.Antivirus,
          rule.ad_domain_joined_required && PostureCheckRequirement.AdJoined,
          rule.windows_security_update_max_age !== null &&
            PostureCheckRequirement.SecurityUpdates,
        ],
      };
    case PostureCheckOs.Macos:
      return {
        summaryParts: [
          mapAnyVersionSummary(rule.min_os_version, (value) => value.toString()),
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
          mapAnyVersionSummary(rule.min_kernel_version, (value) => `Kernel ${value}`),
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
        ],
        filterParts: [
          mapVersionFilterValue(rule.min_kernel_version),
          rule.disk_encryption_required && PostureCheckRequirement.DiskEncryption,
        ],
      };
    case PostureCheckOs.Ios:
      return {
        summaryParts: [
          mapAnyVersionSummary(rule.min_os_version, (value) => `iOS ${value}+`),
        ],
        filterParts: [mapVersionFilterValue(rule.min_os_version)],
      };
    case PostureCheckOs.Android:
      return {
        summaryParts: [
          mapAnyVersionSummary(rule.min_os_version, (value) => `Android ${value}+`),
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
  locations: posture.locations,
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
  defguardDesktop:
    posture.min_desktop_client_version === null
      ? m.posture_checks_version_any()
      : `${posture.min_desktop_client_version}+`,
  defguardDesktopFilters: joinFilters([
    mapVersionFilterValue(posture.min_desktop_client_version),
  ]),
  defguardMobile:
    posture.min_mobile_client_version === null
      ? m.posture_checks_version_any()
      : `${posture.min_mobile_client_version}+`,
  defguardMobileFilters: joinFilters([
    mapVersionFilterValue(posture.min_mobile_client_version),
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

export const getDeletePostureCheckModalData = (
  postureCheck: Pick<PostureCheckRow, 'id' | 'name'>,
  locationNames: string[],
): OpenConfirmActionModal => {
  const formattedLocationNames = formatPostureCheckLocationNames(locationNames);

  return {
    title: m.modal_delete_posture_check_title(),
    contentMd: formattedLocationNames
      ? m.modal_delete_posture_check_content({
          locations: formattedLocationNames,
        })
      : m.modal_delete_posture_check_content_empty(),
    actionPromise: () => api.devicePosture.deleteDevicePosture(postureCheck.id),
    invalidateKeys: [['device-posture'], ['network'], ['activity-log']],
    submitProps: {
      text: m.controls_delete(),
      variant: 'critical',
    },
  };
};

const formatPostureCheckLocationNames = (locationNames: string[]) => {
  if (locationNames.length === 0) {
    return null;
  }

  if (locationNames.length === 1) {
    return locationNames[0];
  }

  if (locationNames.length === 2) {
    return `${locationNames[0]} and ${locationNames[1]}`;
  }

  return `${locationNames.slice(0, -1).join(', ')}, and ${locationNames.at(-1)}`;
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
      row.defguardDesktop,
      row.defguardMobile,
    ];

    return searchableValues.some((value) => value.toLowerCase().includes(query));
  });
};

export const buildFilteredLocationOptions = (locations: NetworkLocation[]) => {
  return locations
    .filter((location) => location.service_location_mode === LocationServiceMode.Disabled)
    .map((loc) => ({
      id: loc.id,
      label: loc.name,
      searchFields: [loc.name, ...loc.address],
    }));
};
