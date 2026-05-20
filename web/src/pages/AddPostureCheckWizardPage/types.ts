import { PostureCheckOs, type PostureCheckOsValue } from '../PostureChecksPage/types';

export const AddPostureCheckWizardStep = {
  OperatingSystems: 'operating-systems',
  ClientVersion: 'client-version',
  Details: 'details',
  Summary: 'summary',
} as const;

export type AddPostureCheckWizardStepValue =
  (typeof AddPostureCheckWizardStep)[keyof typeof AddPostureCheckWizardStep];

export const addPostureCheckWizardStepOrder: AddPostureCheckWizardStepValue[] = [
  AddPostureCheckWizardStep.OperatingSystems,
  AddPostureCheckWizardStep.ClientVersion,
  AddPostureCheckWizardStep.Details,
  AddPostureCheckWizardStep.Summary,
];

export const addPostureCheckOperatingSystems: PostureCheckOsValue[] = [
  PostureCheckOs.Windows,
  PostureCheckOs.Macos,
  PostureCheckOs.Linux,
  PostureCheckOs.Ios,
  PostureCheckOs.Android,
];
