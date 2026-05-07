export type SystemSelectorProps = {
  os: PolicyOsVariant;
  onClick?: () => void;
};

export const PolicyOsVariant = {
  Android: 'android',
  Linux: 'linux',
  Ios: 'ios',
  Windows: 'windows',
  MacOs: 'macos',
} as const;

export type PolicyOsVariant = (typeof PolicyOsVariant)[keyof typeof PolicyOsVariant];
