import { describe, expect, it } from 'vitest';
import { buildAddPostureCheckRequest } from '../src/pages/AddPostureCheckWizardPage/payload';
import { PostureCheckOs } from '../src/pages/PostureChecksPage/types';

describe('add posture check payload', () => {
  const emptyOperatingSystemState = {
    [PostureCheckOs.Windows]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Macos]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Linux]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Ios]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
    [PostureCheckOs.Android]: {
      conditions: [],
      securityUpdateMaxAge: null,
      androidSecurityPatchLevelMaxAge: null,
      version: null,
    },
  };

  it('preserves Any version as null in the API request', () => {
    expect(
      buildAddPostureCheckRequest({
        allowPrereleaseClient: false,
        configuredOperatingSystems: [PostureCheckOs.Windows, PostureCheckOs.Linux],
        description: null,
        desktopAttestation: false,
        desktopManualVerification: false,
        mobileAttestation: false,
        mobileManualVerification: false,
        minimumDesktopClientVersion: null,
        minimumMobileClientVersion: null,
        name: 'Any version policy',
        operatingSystemState: {
          ...emptyOperatingSystemState,
          [PostureCheckOs.Windows]: {
            ...emptyOperatingSystemState[PostureCheckOs.Windows],
            conditions: ['disk-encryption'],
          },
        },
      }),
    ).toEqual({
      name: 'Any version policy',
      description: null,
      min_desktop_client_version: null,
      min_mobile_client_version: null,
      allow_prerelease_client: false,
      desktop_attestation: false,
      desktop_manual_verification: false,
      mobile_attestation: false,
      mobile_manual_verification: false,
      os_rules: [
        {
          os_type: 'windows',
          min_os_version: null,
          disk_encryption_required: true,
          antivirus_required: null,
          ad_domain_joined_required: null,
          windows_security_update_max_age: null,
        },
        {
          os_type: 'linux',
          min_kernel_version: null,
          disk_encryption_required: null,
        },
      ],
    });
  });

  it('sends desktop and mobile client-version requirements independently', () => {
    expect(
      buildAddPostureCheckRequest({
        allowPrereleaseClient: true,
        configuredOperatingSystems: [],
        description: 'Split client requirements',
        desktopAttestation: false,
        desktopManualVerification: false,
        mobileAttestation: false,
        mobileManualVerification: false,
        minimumDesktopClientVersion: '2.1',
        minimumMobileClientVersion: '1.7.0',
        name: 'Client version policy',
        operatingSystemState: emptyOperatingSystemState,
      }),
    ).toEqual({
      name: 'Client version policy',
      description: 'Split client requirements',
      min_desktop_client_version: '2.1',
      min_mobile_client_version: '1.7.0',
      allow_prerelease_client: true,
      desktop_attestation: false,
      desktop_manual_verification: false,
      mobile_attestation: false,
      mobile_manual_verification: false,
      os_rules: [],
    });
  });
  it('sends device attestation and ownership verification flags', () => {
    expect(
      buildAddPostureCheckRequest({
        allowPrereleaseClient: false,
        configuredOperatingSystems: [],
        description: null,
        desktopAttestation: true,
        desktopManualVerification: true,
        mobileAttestation: true,
        mobileManualVerification: false,
        minimumDesktopClientVersion: null,
        minimumMobileClientVersion: null,
        name: 'Attestation policy',
        operatingSystemState: emptyOperatingSystemState,
      }),
    ).toMatchObject({
      desktop_attestation: true,
      desktop_manual_verification: true,
      mobile_attestation: true,
      mobile_manual_verification: false,
    });
  });

  it('drops ownership verification when attestation is disabled', () => {
    expect(
      buildAddPostureCheckRequest({
        allowPrereleaseClient: false,
        configuredOperatingSystems: [],
        description: null,
        desktopAttestation: false,
        desktopManualVerification: true,
        mobileAttestation: false,
        mobileManualVerification: true,
        minimumDesktopClientVersion: null,
        minimumMobileClientVersion: null,
        name: 'Attestation policy',
        operatingSystemState: emptyOperatingSystemState,
      }),
    ).toMatchObject({
      desktop_attestation: false,
      desktop_manual_verification: false,
      mobile_attestation: false,
      mobile_manual_verification: false,
    });
  });
});
