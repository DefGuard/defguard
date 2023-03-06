import { Subject } from 'rxjs';

import { FormStatus, Location } from './types';

export interface ImportedNetwork {
  name: string;
  endpoint: string;
  config: string;
}

export interface WizardStore {
  type?: 'regular' | 'import';
  network: ImportedNetwork;
  stepsCount: number;
  currentStep?: number;
  locations: Location[];
  formStatus: FormStatus;
  editMode: boolean;
  formSubmissionSubject: Subject<number>;
  proceedWizardSubject: Subject<void>;
  addLocation: (location: Location) => void;
  removeLocation: (location: Location) => void;
  setFormStatus: (status: FormStatus) => void;
  setNetwork: (data: ImportedNetwork) => void;
  setState: (data: Partial<WizardStore>) => void;
  resetStore: (data?: Partial<WizardStore>) => void;
}
