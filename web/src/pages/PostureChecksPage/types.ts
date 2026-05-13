export const PostureCheckOs = {
  Windows: 'windows',
  Macos: 'macos',
  Linux: 'linux',
  Ios: 'ios',
  Android: 'android',
} as const;

export type PostureCheckOsValue = (typeof PostureCheckOs)[keyof typeof PostureCheckOs];

export const postureCheckVersionValues = {
  windows: ['Windows 10', 'Windows 11'],
  macos: ['macOS 13 Ventura', 'macOS 14 Sonoma', 'macOS 15 Sequoia', 'macOS 26 Tahoe'],
  linux: ['5.x', '6.x', '7.x'],
  ios: ['17', '18', '26'],
  android: ['13', '14', '15', '16'],
  defguard: ['1.6', '2.0'],
} as const;

export const PostureCheckRequirement = {
  DiskEncryption: 'Disk encryption',
  Antivirus: 'Antivirus',
  AdJoined: 'AD joined',
  SecurityUpdates: 'Security updates',
  DeviceIntegrity: 'Device integrity',
  PrereleaseAllowed: 'Pre-release allowed',
} as const;

type ArrayValues<T extends readonly string[]> = T[number];

export type PostureCheckRequirementValue =
  (typeof PostureCheckRequirement)[keyof typeof PostureCheckRequirement];

export type PostureCheckVersionValue =
  | ArrayValues<(typeof postureCheckVersionValues)['windows']>
  | ArrayValues<(typeof postureCheckVersionValues)['macos']>
  | ArrayValues<(typeof postureCheckVersionValues)['linux']>
  | ArrayValues<(typeof postureCheckVersionValues)['ios']>
  | ArrayValues<(typeof postureCheckVersionValues)['android']>
  | ArrayValues<(typeof postureCheckVersionValues)['defguard']>;

export type PostureCheckDefguardVersionValue = ArrayValues<
  (typeof postureCheckVersionValues)['defguard']
>;

export type PostureCheckFilterValue =
  | PostureCheckVersionValue
  | PostureCheckRequirementValue;
