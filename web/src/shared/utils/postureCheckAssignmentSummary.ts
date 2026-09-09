import { m } from '../../paraglide/messages';
import type { ApiDevicePosture } from '../api/types';
import type { SummarySection } from '../components/SummaryTooltip/type';
import { policyOsVariantToText } from './policyPostures';

const getOsLine = (
  osType: 'windows' | 'macos' | 'linux' | 'ios' | 'android',
  version: number | null,
) => {
  if (version === null) {
    return String(m.posture_checks_version_any());
  }

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
): SummarySection[] => {
  const sections: SummarySection[] = [];

  postureCheck.os_rules.forEach((rule) => {
    switch (rule.os_type) {
      case 'windows': {
        const lines = [
          getOsLine(rule.os_type, rule.min_os_version),
          rule.windows_security_update_max_age !== null
            ? String(
                m.posture_checks_wizard_operating_systems_windows_security_updates_within_days(
                  { days: rule.windows_security_update_max_age },
                ),
              )
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
          getOsLine(rule.os_type, rule.min_os_version),
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
          getOsLine(rule.os_type, rule.min_kernel_version),
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
        const lines = [getOsLine(rule.os_type, rule.min_os_version)].filter(
          (line): line is string => Boolean(line),
        );

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
          getOsLine(rule.os_type, rule.min_os_version),
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
    String(
      m.posture_checks_wizard_summary_desktop_client_version({
        version:
          postureCheck.min_desktop_client_version === null
            ? String(m.posture_checks_version_any())
            : String(
                m.posture_checks_wizard_summary_defguard_version({
                  version: postureCheck.min_desktop_client_version,
                }),
              ),
      }),
    ),
    String(
      m.posture_checks_wizard_summary_mobile_application_version({
        version:
          postureCheck.min_mobile_client_version === null
            ? String(m.posture_checks_version_any())
            : String(
                m.posture_checks_wizard_summary_defguard_version({
                  version: postureCheck.min_mobile_client_version,
                }),
              ),
      }),
    ),
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
