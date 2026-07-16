import { m } from '../../paraglide/messages';
import type { ApiDevicePosture } from '../api/types';
import { IconKind } from '../defguard-ui/components/Icon';

type OsDetailRow = {
  label: string;
  value: string | string[];
};

type OsSection = {
  icon: (typeof IconKind)[keyof typeof IconKind];
  name: string;
  rows: OsDetailRow[];
};

export const getWindowsSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'windows' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.ad_domain_joined_required) otherItems.push('Connected to Active Directory');
  if (rule.antivirus_required) otherItems.push('Antivirus installed');
  if (rule.disk_encryption_required) otherItems.push('Disk encryption enabled');

  const rows: OsDetailRow[] = [];
  rows.push({
    label: 'Version',
    value:
      rule.min_os_version === null
        ? m.posture_checks_version_any()
        : `Windows ${rule.min_os_version} and higher`,
  });
  if (rule.windows_security_update_max_age !== null) {
    rows.push({
      label: 'Update',
      value: `Minimum ${rule.windows_security_update_max_age} month update`,
    });
  }
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Windows, name: 'Windows', rows };
};

export const getMacosSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'macos' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.disk_encryption_required) otherItems.push('Disk encryption enabled');
  if (rule.device_integrity_required) otherItems.push('Device integrity');

  const rows: OsDetailRow[] = [];
  rows.push({
    label: 'Version',
    value:
      rule.min_os_version === null
        ? m.posture_checks_version_any()
        : `macOS ${rule.min_os_version} and higher`,
  });
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Apple, name: 'macOS', rows };
};

export const getLinuxSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'linux' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.disk_encryption_required) otherItems.push('Disk encryption enabled');

  const rows: OsDetailRow[] = [];
  rows.push({
    label: 'Version',
    value:
      rule.min_kernel_version === null
        ? m.posture_checks_version_any()
        : `Kernel ${rule.min_kernel_version} and higher`,
  });
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Linux, name: 'Linux', rows };
};

export const getIosSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'ios' }>,
): OsSection => {
  const rows: OsDetailRow[] = [];
  rows.push({
    label: 'Version',
    value:
      rule.min_os_version === null
        ? m.posture_checks_version_any()
        : `iOS ${rule.min_os_version}+`,
  });

  return { icon: IconKind.AppStore, name: 'iOS', rows };
};

export const getAndroidSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'android' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.device_integrity_required) otherItems.push('Device integrity');

  const rows: OsDetailRow[] = [];
  rows.push({
    label: 'Version',
    value:
      rule.min_os_version === null
        ? m.posture_checks_version_any()
        : `Android ${rule.min_os_version}+`,
  });
  if (rule.android_security_patch_level_max_age !== null) {
    rows.push({
      label: 'Security patch',
      value: `Within ${rule.android_security_patch_level_max_age} days`,
    });
  }
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Android, name: 'Android', rows };
};

export const getDefguardSection = (posture: ApiDevicePosture): OsSection | null => {
  const rows: OsDetailRow[] = [];
  rows.push({
    label: 'Desktop client',
    value:
      posture.min_desktop_client_version === null
        ? m.posture_checks_version_any()
        : `${posture.min_desktop_client_version} and higher`,
  });
  rows.push({
    label: 'Mobile application',
    value:
      posture.min_mobile_client_version === null
        ? m.posture_checks_version_any()
        : `${posture.min_mobile_client_version} and higher`,
  });
  if (posture.allow_prerelease_client) {
    rows.push({ label: 'Other', value: 'Pre-release allowed' });
  }

  return { icon: IconKind.Defguard, name: 'Defguard', rows };
};

export const buildOsSections = (posture: ApiDevicePosture): OsSection[] => {
  const sections: OsSection[] = [];

  for (const rule of posture.os_rules ?? []) {
    switch (rule.os_type) {
      case 'windows':
        sections.push(getWindowsSection(rule));
        break;
      case 'macos':
        sections.push(getMacosSection(rule));
        break;
      case 'linux':
        sections.push(getLinuxSection(rule));
        break;
      case 'ios':
        sections.push(getIosSection(rule));
        break;
      case 'android':
        sections.push(getAndroidSection(rule));
        break;
    }
  }

  const defguardSection = getDefguardSection(posture);
  if (defguardSection) sections.push(defguardSection);

  return sections;
};
