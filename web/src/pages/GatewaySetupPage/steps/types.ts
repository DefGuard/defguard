export type SetupEvent = {
  step: SetupStepId;
  version?: string;
  message?: string;
  logs?: string[];
  error: boolean;
};

export type SetupStep = {
  id: SetupStepId;
  title: string;
};

export type SetupStepId =
  | 'CheckingConfiguration'
  | 'CheckingAvailability'
  | 'CheckingVersion'
  | 'ObtainingCsr'
  | 'SigningCertificate'
  | 'ConfiguringTls'
  | 'Done';
