import { m } from '../../paraglide/messages';
import type { ApiDevicePosture } from '../../shared/api/types';
import { policyOsVariantToText } from '../../shared/utils/policyPostures';

export type PostureChecksSectionState = {
  hasAnyPostureChecks: boolean;
  hasAssignedPostureChecks: boolean;
  locked: boolean;
  showAssignButton: boolean;
  showLockedButton: boolean;
  showAssignedPostureChecks: boolean;
  showEmptyState: boolean;
};

type Input = {
  assignedPostureChecksCount: number;
  canUseEnterprise: boolean | undefined;
  postureChecksCount: number;
};

export type PostureCheckAssignmentSummarySection = {
  label: string;
  lines: string[];
};

const getOsLine = (
  osType: 'windows' | 'macos' | 'linux' | 'ios' | 'android',
  version: number,
) => {
  switch (osType) {
    case 'windows':
    case 'macos':
      return `${policyOsVariantToText(osType)} ${version}+`;
    case 'linux':
      return String(m.posture_checks_wizard_summary_linux_version({ version }));
    case 'ios':
      return String(m.posture_checks_wizard_summary_ios_version({ version }));
    case 'android':
      return String(m.posture_checks_wizard_summary_android_version({ version }));
  }
};

export const getPostureCheckAssignmentSummarySections = (
  postureCheck: ApiDevicePosture,
): PostureCheckAssignmentSummarySection[] => {
  const sections: PostureCheckAssignmentSummarySection[] = [];

  postureCheck.os_rules.forEach((rule) => {
    switch (rule.os_type) {
      case 'windows': {
        const lines = [
          rule.min_os_version !== null
            ? getOsLine(rule.os_type, rule.min_os_version)
            : null,
          rule.windows_security_update_current
            ? String(m.posture_checks_wizard_operating_systems_windows_security_updates())
            : null,
          rule.ad_domain_joined_required
            ? String(
                m.posture_checks_wizard_operating_systems_condition_active_directory(),
              )
            : null,
          rule.antivirus_required
            ? String(m.posture_checks_wizard_operating_systems_condition_antivirus())
            : null,
          rule.disk_encryption_required
            ? String(
                m.posture_checks_wizard_operating_systems_condition_disk_encryption(),
              )
            : null,
        ].filter((line): line is string => Boolean(line));

        if (lines.length > 0) {
          sections.push({
            label: policyOsVariantToText(rule.os_type),
            lines,
          });
        }
        break;
      }
      case 'macos': {
        const lines = [
          rule.min_os_version !== null
            ? getOsLine(rule.os_type, rule.min_os_version)
            : null,
          rule.disk_encryption_required
            ? String(
                m.posture_checks_wizard_operating_systems_condition_disk_encryption(),
              )
            : null,
          rule.device_integrity_required
            ? String(
                m.posture_checks_wizard_operating_systems_condition_device_integrity(),
              )
            : null,
        ].filter((line): line is string => Boolean(line));

        if (lines.length > 0) {
          sections.push({
            label: policyOsVariantToText(rule.os_type),
            lines,
          });
        }
        break;
      }
      case 'linux': {
        const lines = [
          rule.min_kernel_version !== null
            ? getOsLine(rule.os_type, rule.min_kernel_version)
            : null,
          rule.disk_encryption_required
            ? String(
                m.posture_checks_wizard_operating_systems_condition_disk_encryption(),
              )
            : null,
        ].filter((line): line is string => Boolean(line));

        if (lines.length > 0) {
          sections.push({
            label: policyOsVariantToText(rule.os_type),
            lines,
          });
        }
        break;
      }
      case 'ios': {
        const lines = [
          rule.min_os_version !== null
            ? getOsLine(rule.os_type, rule.min_os_version)
            : null,
        ].filter((line): line is string => Boolean(line));

        if (lines.length > 0) {
          sections.push({
            label: policyOsVariantToText(rule.os_type),
            lines,
          });
        }
        break;
      }
      case 'android': {
        const lines = [
          rule.min_os_version !== null
            ? getOsLine(rule.os_type, rule.min_os_version)
            : null,
          rule.device_integrity_required
            ? String(
                m.posture_checks_wizard_operating_systems_condition_device_integrity(),
              )
            : null,
        ].filter((line): line is string => Boolean(line));

        if (lines.length > 0) {
          sections.push({
            label: policyOsVariantToText(rule.os_type),
            lines,
          });
        }
        break;
      }
    }
  });

  const clientLines = [
    postureCheck.min_client_version !== null
      ? String(
          m.posture_checks_wizard_summary_defguard_version({
            version: postureCheck.min_client_version,
          }),
        )
      : null,
    postureCheck.allow_prerelease_client
      ? String(m.posture_checks_wizard_summary_prerelease())
      : null,
  ].filter((line): line is string => Boolean(line));

  if (clientLines.length > 0) {
    sections.push({
      label: String(m.posture_checks_wizard_summary_defguard_label()),
      lines: clientLines,
    });
  }

  return sections;
};

// Keep the edit-location posture-check section aligned with the Figma states.
export const getPostureChecksSectionState = ({
  assignedPostureChecksCount,
  canUseEnterprise,
  postureChecksCount,
}: Input): PostureChecksSectionState => {
  const locked = canUseEnterprise === false;
  const hasAnyPostureChecks = postureChecksCount > 0;
  const hasAssignedPostureChecks = assignedPostureChecksCount > 0;

  return {
    hasAnyPostureChecks,
    hasAssignedPostureChecks,
    locked,
    showAssignButton: !locked && hasAnyPostureChecks && !hasAssignedPostureChecks,
    showLockedButton: locked,
    showAssignedPostureChecks: !locked && hasAssignedPostureChecks,
    showEmptyState: !locked && !hasAnyPostureChecks,
  };
};
