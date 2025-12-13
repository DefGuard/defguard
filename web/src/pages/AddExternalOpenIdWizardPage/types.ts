export const AddExternalProviderStep = {
  ClientSettings: 'client-settings',
  DirectorySync: 'directory-sync',
  Validation: 'validation',
} as const;

export type AddExternalProviderStepValue =
  (typeof AddExternalProviderStep)[keyof typeof AddExternalProviderStep];
