import { beforeEach, describe, expect, it } from 'vitest';
import { useAddPostureCheckWizardStore } from '../src/pages/AddPostureCheckWizardPage/useAddPostureCheckWizardStore';
import {
  getPostureCheckVersionValues,
  PostureCheckOs,
} from '../src/pages/PostureChecksPage/types';
import type { DevicePostureVersionMetadata } from '../src/shared/api/types';

const versionValues = getPostureCheckVersionValues({
  os_versions: {
    windows: ['Windows 10', 'Windows 11'],
    macos: ['macOS 13 Ventura', 'macOS 14 Sonoma', 'macOS 15 Sequoia', 'macOS 26 Tahoe'],
    ios: ['17', '18', '26'],
    android: ['13', '14', '15', '16'],
  },
  linux_kernel_versions: ['5.x', '6.x', '7.x'],
  client_versions: ['1.6', '2.0'],
} satisfies DevicePostureVersionMetadata);

describe('add posture check wizard store', () => {
  beforeEach(() => {
    useAddPostureCheckWizardStore.getState().reset();
    useAddPostureCheckWizardStore.getState().syncVersionValues(versionValues);
  });

  it('stores defguard client-version settings and restores their defaults on reset', () => {
    expect(useAddPostureCheckWizardStore.getState().minimumClientVersion).toBe(
      versionValues.defguard[versionValues.defguard.length - 1],
    );
    expect(useAddPostureCheckWizardStore.getState().allowPrereleaseClient).toBe(false);

    useAddPostureCheckWizardStore.getState().setMinimumClientVersion('1.6');
    useAddPostureCheckWizardStore.getState().setAllowPrereleaseClient(true);

    expect(useAddPostureCheckWizardStore.getState().minimumClientVersion).toBe('1.6');
    expect(useAddPostureCheckWizardStore.getState().allowPrereleaseClient).toBe(true);

    useAddPostureCheckWizardStore.getState().reset();

    expect(useAddPostureCheckWizardStore.getState().minimumClientVersion).toBe(
      versionValues.defguard[versionValues.defguard.length - 1],
    );
    expect(useAddPostureCheckWizardStore.getState().allowPrereleaseClient).toBe(false);
  });

  it('keeps selected operating systems unique while preserving append order', () => {
    useAddPostureCheckWizardStore
      .getState()
      .addConfiguredOperatingSystem(PostureCheckOs.Windows);
    useAddPostureCheckWizardStore
      .getState()
      .addConfiguredOperatingSystem(PostureCheckOs.Macos);
    useAddPostureCheckWizardStore
      .getState()
      .addConfiguredOperatingSystem(PostureCheckOs.Windows);

    expect(useAddPostureCheckWizardStore.getState().configuredOperatingSystems).toEqual([
      PostureCheckOs.Windows,
      PostureCheckOs.Macos,
    ]);
  });

  it('persists operating-system details when navigating away from and back to step 1', () => {
    const store = useAddPostureCheckWizardStore.getState();

    store.addConfiguredOperatingSystem(PostureCheckOs.Windows);
    store.updateOperatingSystemDetails(PostureCheckOs.Windows, {
      conditions: ['disk-encryption', 'antivirus'],
      securityUpdates: true,
      version: 'Windows 10',
    });

    store.next();
    store.back();

    expect(useAddPostureCheckWizardStore.getState().operatingSystemState.windows).toEqual(
      {
        conditions: ['disk-encryption', 'antivirus'],
        securityUpdates: true,
        version: 'Windows 10',
      },
    );
  });

  it('stores posture-check details and restores their defaults on reset', () => {
    const store = useAddPostureCheckWizardStore.getState();

    expect(store.name).toBe('');
    expect(store.description).toBeNull();

    store.setName('Windows production policy');
    store.setDescription('Restricts unmanaged Windows devices to client version 2.0+.');

    expect(useAddPostureCheckWizardStore.getState().name).toBe(
      'Windows production policy',
    );
    expect(useAddPostureCheckWizardStore.getState().description).toBe(
      'Restricts unmanaged Windows devices to client version 2.0+.',
    );

    store.reset();

    expect(useAddPostureCheckWizardStore.getState().name).toBe('');
    expect(useAddPostureCheckWizardStore.getState().description).toBeNull();
  });
});
