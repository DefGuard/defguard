import type {
  EditDevicePostureOsRule,
  EditDevicePostureRequest,
} from '../../shared/api/types';
import {
  type PostureCheckDefguardVersionValue,
  PostureCheckOs,
  type PostureCheckOsValue,
} from '../PostureChecksPage/types';
import type {
  OperatingSystemConditionKey,
  OperatingSystemFormState,
} from './useAddPostureCheckWizardStore';

type BuildAddPostureCheckRequestInput = {
  allowPrereleaseClient: boolean;
  configuredOperatingSystems: PostureCheckOsValue[];
  description: string | null;
  minimumClientVersion: PostureCheckDefguardVersionValue;
  name: string;
  operatingSystemState: Record<PostureCheckOsValue, OperatingSystemFormState>;
};

const hasCondition = (
  conditions: OperatingSystemConditionKey[],
  condition: OperatingSystemConditionKey,
) => conditions.includes(condition);

const buildOperatingSystemRule = (
  operatingSystem: PostureCheckOsValue,
  details: OperatingSystemFormState,
): EditDevicePostureOsRule => {
  switch (operatingSystem) {
    case PostureCheckOs.Windows:
      return {
        os_type: PostureCheckOs.Windows,
        min_os_version: details.version,
        disk_encryption_required: hasCondition(details.conditions, 'disk-encryption')
          ? true
          : null,
        antivirus_required: hasCondition(details.conditions, 'antivirus') ? true : null,
        ad_domain_joined_required: hasCondition(details.conditions, 'active-directory')
          ? true
          : null,
        windows_security_update_current: details.securityUpdates ? true : null,
      };
    case PostureCheckOs.Macos:
      return {
        os_type: PostureCheckOs.Macos,
        min_os_version: details.version,
        disk_encryption_required: hasCondition(details.conditions, 'disk-encryption')
          ? true
          : null,
        device_integrity_required: hasCondition(details.conditions, 'device-integrity')
          ? true
          : null,
      };
    case PostureCheckOs.Linux:
      return {
        os_type: PostureCheckOs.Linux,
        min_kernel_version: details.version,
        disk_encryption_required: hasCondition(details.conditions, 'disk-encryption')
          ? true
          : null,
      };
    case PostureCheckOs.Ios:
      return {
        os_type: PostureCheckOs.Ios,
        min_os_version: details.version,
      };
    case PostureCheckOs.Android:
      return {
        os_type: PostureCheckOs.Android,
        min_os_version: details.version,
        device_integrity_required: hasCondition(details.conditions, 'device-integrity')
          ? true
          : null,
      };
  }
};

export const buildAddPostureCheckRequest = (
  input: BuildAddPostureCheckRequestInput,
): EditDevicePostureRequest => ({
  name: input.name,
  description: input.description,
  min_client_version: input.minimumClientVersion,
  allow_prerelease_client: input.allowPrereleaseClient,
  os_rules: input.configuredOperatingSystems.map((operatingSystem) =>
    buildOperatingSystemRule(
      operatingSystem,
      input.operatingSystemState[operatingSystem],
    ),
  ),
});
