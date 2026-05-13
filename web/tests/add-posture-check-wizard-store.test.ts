import { beforeEach, describe, expect, it } from 'vitest';
import { useAddPostureCheckWizardStore } from '../src/pages/AddPostureCheckWizardPage/useAddPostureCheckWizardStore';
import {
  PostureCheckOs,
  postureCheckVersionValues,
} from '../src/pages/PostureChecksPage/types';

describe('add posture check wizard store', () => {
  beforeEach(() => {
    useAddPostureCheckWizardStore.getState().reset();
  });

  it('stores defguard client-version settings and restores their defaults on reset', () => {
    expect(useAddPostureCheckWizardStore.getState().minimumClientVersion).toBe(
      postureCheckVersionValues.defguard[postureCheckVersionValues.defguard.length - 1],
    );
    expect(useAddPostureCheckWizardStore.getState().allowPrereleaseClient).toBe(false);

    useAddPostureCheckWizardStore.getState().setMinimumClientVersion('1.6');
    useAddPostureCheckWizardStore.getState().setAllowPrereleaseClient(true);

    expect(useAddPostureCheckWizardStore.getState().minimumClientVersion).toBe('1.6');
    expect(useAddPostureCheckWizardStore.getState().allowPrereleaseClient).toBe(true);

    useAddPostureCheckWizardStore.getState().reset();

    expect(useAddPostureCheckWizardStore.getState().minimumClientVersion).toBe(
      postureCheckVersionValues.defguard[postureCheckVersionValues.defguard.length - 1],
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
