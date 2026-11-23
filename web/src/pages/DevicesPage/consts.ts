import { m } from '../../paraglide/messages';
import {
  DeviceAttestationStatus,
  type DeviceAttestationStatusValue,
  DevicePlatform,
  type DevicePlatformValue,
  DeviceVerification,
  type DeviceVerificationValue,
} from '../../shared/api/types';
import type { PolicyOsVariant } from '../../shared/components/SystemSelector/types';

export const osIconSize = 20;

export const mobileOsTypes: PolicyOsVariant[] = ['android', 'ios'];

export const devicePlatformText: Record<DevicePlatformValue, () => string> = {
  [DevicePlatform.Desktop]: m.devices_platform_desktop,
  [DevicePlatform.Mobile]: m.devices_platform_mobile,
  [DevicePlatform.Unknown]: m.devices_unknown,
};

export const attestationStatusText: Record<DeviceAttestationStatusValue, () => string> = {
  [DeviceAttestationStatus.NotAttested]: m.devices_attestation_not_attested,
  [DeviceAttestationStatus.Attested]: m.devices_attestation_attested,
  [DeviceAttestationStatus.Unrecognized]: m.devices_attestation_unrecognized,
  [DeviceAttestationStatus.AttestedManually]: m.devices_attestation_attested_manually,
  [DeviceAttestationStatus.AttestationFailed]: m.devices_attestation_failed,
};

export const verificationText: Record<DeviceVerificationValue, () => string> = {
  [DeviceVerification.Verified]: m.devices_verification_verified,
  [DeviceVerification.Unverified]: m.devices_verification_unverified,
  [DeviceVerification.Pending]: m.devices_verification_pending,
};
