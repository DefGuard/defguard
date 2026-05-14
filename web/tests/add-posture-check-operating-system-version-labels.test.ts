import { describe, expect, it } from 'vitest';
import { getOperatingSystemVersionOptionLabel } from '../src/pages/AddPostureCheckWizardPage/operatingSystemVersionLabels';
import { PostureCheckOs } from '../src/pages/PostureChecksPage/types';

describe('add posture check operating-system version labels', () => {
  it('includes explicit platform names for operating-system version thresholds', () => {
    expect(getOperatingSystemVersionOptionLabel(PostureCheckOs.Windows, 11)).toBe(
      'Windows 11 or higher',
    );
    expect(getOperatingSystemVersionOptionLabel(PostureCheckOs.Macos, 26)).toBe(
      'macOS 26 or higher',
    );
    expect(getOperatingSystemVersionOptionLabel(PostureCheckOs.Linux, 6)).toBe(
      'Kernel 6 or higher',
    );
    expect(getOperatingSystemVersionOptionLabel(PostureCheckOs.Ios, 18)).toBe(
      'iOS 18 or higher',
    );
    expect(getOperatingSystemVersionOptionLabel(PostureCheckOs.Android, 16)).toBe(
      'Android 16 or higher',
    );
  });
});
