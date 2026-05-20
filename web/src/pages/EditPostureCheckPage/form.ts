import type { ApiDevicePosture, ApiDevicePostureOsRule } from '../../shared/api/types';
import type { OperatingSystemConditionKey } from '../AddPostureCheckWizardPage/useAddPostureCheckWizardStore';
import {
  PostureCheckOs,
  type PostureCheckOsValue,
  type PostureCheckVersionValues,
} from '../PostureChecksPage/types';

export type EditPostureCheckOperatingSystemState = {
  conditions: OperatingSystemConditionKey[];
  securityUpdates: boolean;
  version: number | null;
};

export type EditPostureCheckFormValues = {
  allowPrereleaseClient: boolean;
  configuredOperatingSystems: PostureCheckOsValue[];
  description: string | null;
  locations: Set<number>;
  minimumClientVersion: string;
  name: string;
  operatingSystemState: Record<PostureCheckOsValue, EditPostureCheckOperatingSystemState>;
};

export const editPostureCheckOperatingSystems: PostureCheckOsValue[] = [
  PostureCheckOs.Windows,
  PostureCheckOs.Macos,
  PostureCheckOs.Linux,
  PostureCheckOs.Ios,
  PostureCheckOs.Android,
];

export const getDefaultEditPostureCheckOperatingSystemState = (
  versionValues: PostureCheckVersionValues,
): Record<PostureCheckOsValue, EditPostureCheckOperatingSystemState> => ({
  [PostureCheckOs.Windows]: {
    conditions: [],
    securityUpdates: false,
    version: versionValues.windows[versionValues.windows.length - 1] ?? null,
  },
  [PostureCheckOs.Macos]: {
    conditions: [],
    securityUpdates: false,
    version: versionValues.macos[versionValues.macos.length - 1] ?? null,
  },
  [PostureCheckOs.Linux]: {
    conditions: [],
    securityUpdates: false,
    version: versionValues.linux[versionValues.linux.length - 1] ?? null,
  },
  [PostureCheckOs.Ios]: {
    conditions: [],
    securityUpdates: false,
    version: versionValues.ios[versionValues.ios.length - 1] ?? null,
  },
  [PostureCheckOs.Android]: {
    conditions: [],
    securityUpdates: false,
    version: versionValues.android[versionValues.android.length - 1] ?? null,
  },
});

const getRuleConditions = (
  rule: ApiDevicePostureOsRule,
): OperatingSystemConditionKey[] => {
  switch (rule.os_type) {
    case PostureCheckOs.Windows:
      return [
        rule.ad_domain_joined_required ? 'active-directory' : null,
        rule.antivirus_required ? 'antivirus' : null,
        rule.disk_encryption_required ? 'disk-encryption' : null,
      ].filter((value): value is OperatingSystemConditionKey => value !== null);
    case PostureCheckOs.Macos:
      return [
        rule.disk_encryption_required ? 'disk-encryption' : null,
        rule.device_integrity_required ? 'device-integrity' : null,
      ].filter((value): value is OperatingSystemConditionKey => value !== null);
    case PostureCheckOs.Linux:
      return [rule.disk_encryption_required ? 'disk-encryption' : null].filter(
        (value): value is OperatingSystemConditionKey => value !== null,
      );
    case PostureCheckOs.Android:
      return [rule.device_integrity_required ? 'device-integrity' : null].filter(
        (value): value is OperatingSystemConditionKey => value !== null,
      );
    default:
      return [];
  }
};

const getRuleVersion = (rule: ApiDevicePostureOsRule): number | null => {
  switch (rule.os_type) {
    case PostureCheckOs.Linux:
      return rule.min_kernel_version;
    default:
      return rule.min_os_version;
  }
};

export const getInitialEditPostureCheckFormValues = (
  postureCheck: ApiDevicePosture,
  versionValues: PostureCheckVersionValues,
): EditPostureCheckFormValues => {
  const operatingSystemState =
    getDefaultEditPostureCheckOperatingSystemState(versionValues);

  for (const rule of postureCheck.os_rules) {
    operatingSystemState[rule.os_type] = {
      conditions: getRuleConditions(rule),
      securityUpdates:
        rule.os_type === PostureCheckOs.Windows &&
        rule.windows_security_update_current === true,
      version: getRuleVersion(rule),
    };
  }

  return {
    allowPrereleaseClient: postureCheck.allow_prerelease_client,
    configuredOperatingSystems: postureCheck.os_rules.map((rule) => rule.os_type),
    description: postureCheck.description,
    locations: new Set(postureCheck.locations),
    minimumClientVersion:
      postureCheck.min_client_version ??
      versionValues.defguard[versionValues.defguard.length - 1] ??
      '',
    name: postureCheck.name,
    operatingSystemState,
  };
};

export const normalizeEditPostureCheckFormValues = (
  values: EditPostureCheckFormValues,
) => ({
  ...values,
  locations: Array.from(values.locations).sort((left, right) => left - right),
});
