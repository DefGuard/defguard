import type { ApiDevicePosture, ApiDevicePostureOsRule } from '../../shared/api/types';
import type { OperatingSystemConditionKey } from '../AddPostureCheckWizardPage/useAddPostureCheckWizardStore';
import { PostureCheckOs, type PostureCheckOsValue } from '../PostureChecksPage/types';

export type EditPostureCheckOperatingSystemState = {
  conditions: OperatingSystemConditionKey[];
  /** Windows only */
  securityUpdateMaxAge: number | null;
  /** Android only */
  androidSecurityPatchLevelMaxAge: number | null;
  version: number | null;
};

export type EditPostureCheckFormValues = {
  allowPrereleaseClient: boolean;
  configuredOperatingSystems: PostureCheckOsValue[];
  description: string | null;
  locations: Set<number>;
  minimumDesktopClientVersion: string | null;
  minimumMobileClientVersion: string | null;
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

export const getDefaultEditPostureCheckOperatingSystemState = (): Record<
  PostureCheckOsValue,
  EditPostureCheckOperatingSystemState
> => ({
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
): EditPostureCheckFormValues => {
  const operatingSystemState = getDefaultEditPostureCheckOperatingSystemState();

  for (const rule of postureCheck.os_rules) {
    operatingSystemState[rule.os_type] = {
      conditions: getRuleConditions(rule),
      securityUpdateMaxAge:
        rule.os_type === PostureCheckOs.Windows
          ? rule.windows_security_update_max_age
          : null,
      androidSecurityPatchLevelMaxAge:
        rule.os_type === PostureCheckOs.Android
          ? rule.android_security_patch_level_max_age
          : null,
      version: getRuleVersion(rule),
    };
  }

  return {
    allowPrereleaseClient: postureCheck.allow_prerelease_client,
    configuredOperatingSystems: postureCheck.os_rules.map((rule) => rule.os_type),
    description: postureCheck.description,
    locations: new Set(postureCheck.locations),
    minimumDesktopClientVersion: postureCheck.min_desktop_client_version,
    minimumMobileClientVersion: postureCheck.min_mobile_client_version,
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
