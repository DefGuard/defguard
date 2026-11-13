import { m } from '../../paraglide/messages';
import {
  DeviceAttestationStatus,
  type DeviceAttestationStatusValue,
  DevicePlatform,
  type DevicePlatformValue,
  DeviceState,
  type DeviceStateValue,
  DeviceVerification,
  type DeviceVerificationValue,
} from '../../shared/api/types';
import type { PolicyOsVariant } from '../../shared/components/SystemSelector/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../shared/defguard-ui/components/Badge/types';
import type { IconKindValue } from '../../shared/defguard-ui/components/Icon';
import { policyOsVariantToText } from '../../shared/utils/policyPostures';

export const osIconSize = 20;

export const unknownBadge = (
  <Badge variant={BadgeVariant.Neutral} text={m.devices_unknown()} />
);

const mobileOsTypes: PolicyOsVariant[] = ['android', 'ios'];

const isMobileOs = (osType: PolicyOsVariant) => mobileOsTypes.includes(osType);

export const platformIcon = (osType: PolicyOsVariant): IconKindValue =>
  isMobileOs(osType) ? 'mobile' : 'desktop';

export const platformText = (osType: PolicyOsVariant) =>
  isMobileOs(osType) ? m.devices_platform_mobile() : m.devices_platform_desktop();

export const devicePlatformText: Record<DevicePlatformValue, () => string> = {
  [DevicePlatform.Desktop]: m.devices_platform_desktop,
  [DevicePlatform.Mobile]: m.devices_platform_mobile,
  [DevicePlatform.Unknown]: m.devices_unknown,
};

export const systemText = (osType: PolicyOsVariant, osVersion: string | null) =>
  [policyOsVariantToText(osType), osVersion].filter(Boolean).join(' ');

export const blockedBadge = (
  <Badge
    variant={BadgeVariant.Critical}
    icon="status-locked"
    showIcon
    text={m.devices_state_blocked()}
  />
);

type DeviceStateBadgeProps = {
  state: DeviceStateValue;
  removeBackground?: boolean;
};

export const DeviceStateBadge = ({ state, removeBackground }: DeviceStateBadgeProps) => {
  if (state === DeviceState.Blocked) return blockedBadge;
  return (
    <Badge
      variant={BadgeVariant.Success}
      showIcon
      removeBackground={removeBackground}
      text={m.state_enabled()}
    />
  );
};

type ConnectionBadgeProps = {
  connected: boolean;
  removeBackground?: boolean;
};

export const ConnectionBadge = ({
  connected,
  removeBackground,
}: ConnectionBadgeProps) => {
  if (connected)
    return (
      <Badge
        variant={BadgeVariant.Success}
        showIcon
        removeBackground={removeBackground}
        text={m.devices_connection_online()}
      />
    );
  return (
    <Badge
      variant={BadgeVariant.Neutral}
      showIcon
      removeBackground
      text={m.devices_connection_offline()}
    />
  );
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
