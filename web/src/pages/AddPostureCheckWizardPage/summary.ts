import { m } from '../../paraglide/messages';
import type { IconKindValue } from '../../shared/defguard-ui/components/Icon';
import {
  policyOsVariantToIcon,
  policyOsVariantToText,
} from '../../shared/utils/policyPostures';
import {
  type PostureCheckDefguardVersionValue,
  PostureCheckOs,
  type PostureCheckOsValue,
  type PostureCheckOsVersionValue,
} from '../PostureChecksPage/types';
import type {
  OperatingSystemConditionKey,
  OperatingSystemFormState,
} from './useAddPostureCheckWizardStore';

export type SummaryLine = {
  text: string;
  emphasized?: boolean;
};

export type SummarySection = {
  icon: IconKindValue;
  label: string;
  lines: SummaryLine[];
};

const getConditionLabel = (condition: OperatingSystemConditionKey) => {
  switch (condition) {
    case 'active-directory':
      return m.posture_checks_wizard_operating_systems_condition_active_directory();
    case 'antivirus':
      return m.posture_checks_wizard_operating_systems_condition_antivirus();
    case 'device-integrity':
      return m.posture_checks_wizard_operating_systems_condition_device_integrity();
    case 'disk-encryption':
      return m.posture_checks_wizard_operating_systems_condition_disk_encryption();
    case 'pre-release':
      return m.posture_checks_wizard_summary_prerelease();
  }
};

const getOperatingSystemVersionLabel = (
  operatingSystem: PostureCheckOsValue,
  version: PostureCheckOsVersionValue,
) => {
  switch (operatingSystem) {
    case PostureCheckOs.Windows:
      return m.posture_checks_wizard_client_version_option({ version });
    case PostureCheckOs.Macos:
      return m.posture_checks_wizard_client_version_option({ version });
    case PostureCheckOs.Linux:
      return m.posture_checks_wizard_summary_linux_version({ version });
    case PostureCheckOs.Ios:
      return m.posture_checks_wizard_summary_ios_version({ version });
    case PostureCheckOs.Android:
      return m.posture_checks_wizard_summary_android_version({ version });
  }
};

export const buildOperatingSystemSummarySection = (
  operatingSystem: PostureCheckOsValue,
  details: OperatingSystemFormState,
): SummarySection => {
  const lines: SummaryLine[] = [];

  if (details.version !== null) {
    lines.push({
      text: getOperatingSystemVersionLabel(operatingSystem, details.version),
      emphasized: true,
    });
  }

  details.conditions.forEach((condition) => {
    lines.push({ text: getConditionLabel(condition) });
  });

  return {
    icon: policyOsVariantToIcon(operatingSystem),
    label: policyOsVariantToText(operatingSystem),
    lines,
  };
};

export const buildClientSummarySection = (
  minimumClientVersion: PostureCheckDefguardVersionValue,
  allowPrereleaseClient: boolean,
): SummarySection => {
  const lines: SummaryLine[] = [
    {
      text: m.posture_checks_wizard_summary_defguard_version({
        version: minimumClientVersion,
      }),
      emphasized: true,
    },
  ];

  if (allowPrereleaseClient) {
    lines.push({ text: getConditionLabel('pre-release') });
  }

  return {
    icon: 'defguard',
    label: m.posture_checks_wizard_summary_defguard_label(),
    lines,
  };
};
