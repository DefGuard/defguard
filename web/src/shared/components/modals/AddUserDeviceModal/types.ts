export const AddUserDeviceModalStep = {
  StartChoice: 'start-choice',
  ManualSetup: 'manual-setup',
  ClientSetup: 'client-setup',
  ManualConfiguration: 'manual-configuration',
} as const;

export type AddUserDeviceModalStepValue =
  (typeof AddUserDeviceModalStep)[keyof typeof AddUserDeviceModalStep];
