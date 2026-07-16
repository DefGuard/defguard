import { describe, expect, it } from 'vitest';
import {
  buildClientSummarySection,
  buildOperatingSystemSummarySection,
} from '../src/pages/AddPostureCheckWizardPage/summary';
import { PostureCheckOs } from '../src/pages/PostureChecksPage/types';

describe('add posture check summary helpers', () => {
  it('builds a windows summary section from the selected operating-system requirements', () => {
    expect(
      buildOperatingSystemSummarySection(PostureCheckOs.Windows, {
        conditions: ['active-directory', 'antivirus'],
        securityUpdateMaxAge: null,
        androidSecurityPatchLevelMaxAge: null,
        version: 10,
      }),
    ).toEqual({
      icon: 'windows',
      label: 'Windows',
      lines: [
        { emphasized: true, text: '10 and higher' },
        { text: 'Connected to Active Directory' },
        { text: 'Antivirus installed' },
      ],
    });
  });

  it('builds the defguard summary section from the client-version settings', () => {
    expect(buildClientSummarySection('2.0', '1.7.0', true)).toEqual({
      icon: 'defguard',
      label: 'Defguard',
      lines: [
        { emphasized: true, text: 'Desktop client: 2.0 and higher' },
        { emphasized: true, text: 'Mobile app: 1.7.0 and higher' },
        { text: 'Allow pre-release versions of the Defguard client.' },
      ],
    });
  });

  it('shows Any version when version requirements are disabled', () => {
    expect(
      buildOperatingSystemSummarySection(PostureCheckOs.Windows, {
        conditions: ['disk-encryption'],
        securityUpdateMaxAge: null,
        androidSecurityPatchLevelMaxAge: null,
        version: null,
      }),
    ).toEqual({
      icon: 'windows',
      label: 'Windows',
      lines: [
        { emphasized: true, text: 'Any version' },
        { text: 'Disk encryption enabled' },
      ],
    });

    expect(buildClientSummarySection(null, null, false)).toEqual({
      icon: 'defguard',
      label: 'Defguard',
      lines: [
        { emphasized: true, text: 'Desktop client: Any version' },
        { emphasized: true, text: 'Mobile app: Any version' },
      ],
    });
  });
});
