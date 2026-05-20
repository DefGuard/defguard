import type { DevicePostureVersionMetadata } from '../../shared/api/types';

export const PostureCheckOs = {
  Windows: 'windows',
  Macos: 'macos',
  Linux: 'linux',
  Ios: 'ios',
  Android: 'android',
} as const;

export type PostureCheckOsValue = (typeof PostureCheckOs)[keyof typeof PostureCheckOs];

export type PostureCheckOsVersionValue = number;
export type PostureCheckDefguardVersionValue = string;

export type PostureCheckVersionValues = Record<
  PostureCheckOsValue,
  readonly PostureCheckOsVersionValue[]
> & {
  defguard: readonly PostureCheckDefguardVersionValue[];
};

export const getPostureCheckVersionValues = (
  metadata: DevicePostureVersionMetadata,
): PostureCheckVersionValues => ({
  windows: metadata.os_versions.windows,
  macos: metadata.os_versions.macos,
  linux: metadata.linux_kernel_versions,
  ios: metadata.os_versions.ios,
  android: metadata.os_versions.android,
  defguard: metadata.client_versions,
});

export const PostureCheckRequirement = {
  DiskEncryption: 'Disk encryption',
  Antivirus: 'Antivirus',
  AdJoined: 'AD joined',
  SecurityUpdates: 'Security updates',
  DeviceIntegrity: 'Device integrity',
  PrereleaseAllowed: 'Pre-release allowed',
} as const;

export type PostureCheckRequirementValue =
  (typeof PostureCheckRequirement)[keyof typeof PostureCheckRequirement];

export type PostureCheckVersionValue = PostureCheckOsVersionValue;

export type PostureCheckFilterValue = string | number;
