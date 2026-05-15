import { isValidElement, type ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import type { ApiDevicePosture } from '../src/shared/api/types';

vi.mock('../src/paraglide/messages', () => ({
  m: {
    posture_checks_wizard_summary_linux_version: ({ version }: { version: number }) =>
      `Linux kernel ${version}+`,
    posture_checks_wizard_summary_ios_version: ({ version }: { version: number }) =>
      `iOS ${version}+`,
    posture_checks_wizard_summary_android_version: ({ version }: { version: number }) =>
      `Android ${version}+`,
    posture_checks_wizard_operating_systems_windows_security_updates: () =>
      'Check security updates',
    posture_checks_wizard_operating_systems_condition_active_directory: () =>
      'Connected to Active Directory',
    posture_checks_wizard_operating_systems_condition_antivirus: () =>
      'Antivirus installed',
    posture_checks_wizard_operating_systems_condition_disk_encryption: () =>
      'Disk encryption enabled',
    posture_checks_wizard_operating_systems_condition_device_integrity: () =>
      'Device integrity enabled',
    posture_checks_wizard_summary_defguard_version: ({ version }: { version: string }) =>
      `Defguard ${version} and higher`,
    posture_checks_wizard_summary_prerelease: () =>
      'Allow pre-release versions of the Defguard client.',
    posture_checks_wizard_summary_defguard_label: () => 'Defguard',
  },
}));

const { getPostureCheckAssignmentSummarySections, getPostureChecksSectionState } =
  await import('../src/pages/EditLocationPage/postureChecksSection');
const { renderPostureCheckSelectionItem } = await import(
  '../src/pages/EditLocationPage/postureCheckSelectionItem'
);

describe('edit location posture-checks section state', () => {
  it('shows the empty state when enterprise access is available but no posture checks exist', () => {
    expect(
      getPostureChecksSectionState({
        assignedPostureChecksCount: 0,
        canUseEnterprise: true,
        postureChecksCount: 0,
      }),
    ).toEqual({
      hasAnyPostureChecks: false,
      hasAssignedPostureChecks: false,
      locked: false,
      showAssignButton: false,
      showLockedButton: false,
      showAssignedPostureChecks: false,
      showEmptyState: true,
    });
    expect(
      getPostureChecksSectionState({
        assignedPostureChecksCount: 0,
        canUseEnterprise: undefined,
        postureChecksCount: 0,
      }),
    ).toEqual({
      hasAnyPostureChecks: false,
      hasAssignedPostureChecks: false,
      locked: false,
      showAssignButton: false,
      showLockedButton: false,
      showAssignedPostureChecks: false,
      showEmptyState: true,
    });
  });

  it('shows the assign button when posture checks exist but none are attached to the location', () => {
    expect(
      getPostureChecksSectionState({
        assignedPostureChecksCount: 0,
        canUseEnterprise: true,
        postureChecksCount: 2,
      }),
    ).toEqual({
      hasAnyPostureChecks: true,
      hasAssignedPostureChecks: false,
      locked: false,
      showAssignButton: true,
      showLockedButton: false,
      showAssignedPostureChecks: false,
      showEmptyState: false,
    });
  });

  it('shows the assigned posture-check list when the location already has posture checks attached', () => {
    expect(
      getPostureChecksSectionState({
        assignedPostureChecksCount: 2,
        canUseEnterprise: true,
        postureChecksCount: 3,
      }),
    ).toEqual({
      hasAnyPostureChecks: true,
      hasAssignedPostureChecks: true,
      locked: false,
      showAssignButton: false,
      showLockedButton: false,
      showAssignedPostureChecks: true,
      showEmptyState: false,
    });
  });

  it('locks the add posture-check CTA when enterprise access is unavailable', () => {
    expect(
      getPostureChecksSectionState({
        assignedPostureChecksCount: 0,
        canUseEnterprise: false,
        postureChecksCount: 2,
      }),
    ).toEqual({
      hasAnyPostureChecks: true,
      hasAssignedPostureChecks: false,
      locked: true,
      showAssignButton: false,
      showLockedButton: true,
      showAssignedPostureChecks: false,
      showEmptyState: false,
    });
  });

  it('builds structured posture-check info sections for the assignment tooltip', () => {
    const postureCheck: ApiDevicePosture = {
      id: 1,
      name: 'Windows admins',
      description: null,
      min_client_version: '2.0',
      allow_prerelease_client: true,
      locations: [],
      os_rules: [
        {
          os_type: 'windows',
          min_os_version: 11,
          disk_encryption_required: true,
          antivirus_required: true,
          ad_domain_joined_required: true,
          windows_security_update_current: true,
        },
        {
          os_type: 'macos',
          min_os_version: 15,
          disk_encryption_required: true,
          device_integrity_required: true,
        },
      ],
    };

    expect(getPostureCheckAssignmentSummarySections(postureCheck)).toEqual([
      {
        label: 'Windows',
        lines: [
          'Windows 11+',
          'Check security updates',
          'Connected to Active Directory',
          'Antivirus installed',
          'Disk encryption enabled',
        ],
      },
      {
        label: 'macOS',
        lines: ['macOS 15+', 'Disk encryption enabled', 'Device integrity enabled'],
      },
      {
        label: 'Defguard',
        lines: [
          'Defguard 2.0 and higher',
          'Allow pre-release versions of the Defguard client.',
        ],
      },
    ]);
  });

  it('keeps the selection row checkbox wired to the provided toggle handler', () => {
    const onClick = vi.fn();
    const option = {
      id: 1,
      label: 'Windows admins',
      meta: {
        id: 1,
        name: 'Windows admins',
        description: null,
        min_client_version: '2.0',
        allow_prerelease_client: false,
        locations: [],
        os_rules: [
          {
            os_type: 'windows' as const,
            min_os_version: 11,
            disk_encryption_required: true,
            antivirus_required: false,
            ad_domain_joined_required: false,
            windows_security_update_current: false,
          },
        ],
      } satisfies ApiDevicePosture,
    };

    const rendered = renderPostureCheckSelectionItem({
      active: true,
      onClick,
      option,
    });

    expect(isValidElement<{ children: ReactNode }>(rendered)).toBe(true);
    if (!isValidElement<{ children: ReactNode }>(rendered)) {
      throw new Error('Expected a posture-check selection row element');
    }

    const checkboxElement = rendered.props.children;

    expect(
      isValidElement<{ onClick: (event: { currentTarget: null }) => void }>(
        checkboxElement,
      ),
    ).toBe(true);
    if (
      !isValidElement<{ onClick: (event: { currentTarget: null }) => void }>(
        checkboxElement,
      )
    ) {
      throw new Error('Expected the row to render a checkbox');
    }

    checkboxElement.props.onClick({ currentTarget: null });

    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
