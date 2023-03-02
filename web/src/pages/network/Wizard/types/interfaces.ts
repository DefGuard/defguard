import { BehaviorSubject, Subject } from 'rxjs';

import { FormStatus, Location, User, WizardNetwork } from './types';

export interface WizardStore {
  stepsCount: number;
  currentStep?: number;
  network: BehaviorSubject<WizardNetwork>;
  users: User[];
  locations: Location[];
  formStatus: FormStatus;
  editMode: boolean;
  formSubmissionSubject: Subject<number>;
  proceedWizardSubject: Subject<void>;
  addLocation: (location: Location) => void;
  removeLocation: (location: Location) => void;
  addUser: (user: User) => void;
  removeUser: (user: User) => void;
  setNetwork: (network: Partial<WizardNetwork>) => void;
  setFormStatus: (status: FormStatus) => void;
  setState: (data: Partial<WizardStore>) => void;
  resetStore: (data?: Partial<WizardStore>) => void;
}
