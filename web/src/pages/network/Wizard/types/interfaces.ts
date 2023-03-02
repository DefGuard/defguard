import { Subject } from 'rxjs';

import { FormStatus, Location } from './types';

export interface WizardStore {
  type?: 'regular' | 'import';
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
  setState: (data: Partial<WizardStore>) => void;
  resetStore: (data?: Partial<WizardStore>) => void;
}
