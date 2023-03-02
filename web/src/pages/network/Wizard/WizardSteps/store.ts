import { Subject } from 'rxjs';
import create from 'zustand';
import { devtools } from 'zustand/middleware';

import { WizardStore } from '../types/interfaces';
import { FormStatus, Location } from '../types/types';

const defaultState = {
  type: undefined,
  stepsCount: 2,
  locations: [],
  editMode: false,
  formStatus: {
    1: false,
    2: false,
    3: false,
  },
};

export const useWizardStore = create<
  WizardStore,
  [['zustand/devtools', WizardStore]]
>(
  devtools((set) => ({
    type: undefined,
    stepsCount: 2,
    locations: [],
    editMode: false,
    formStatus: {
      1: false,
      2: false,
      3: false,
    },
    formSubmissionSubject: new Subject<number>(),
    proceedWizardSubject: new Subject<void>(),
    addLocation: (location: Location) =>
      set((state) => ({ locations: [...state.locations, location] })),
    removeLocation: (location: Location) =>
      set((state) => ({
        locations: state.locations.filter((l) => l.name !== location.name),
      })),
    setFormStatus: (formStatus: FormStatus) =>
      set((state) => ({
        formStatus: { ...state.formStatus, ...formStatus },
      })),
    setState: (data) => set((state) => ({ ...state, ...data })),
    resetStore: (data) =>
      set((state) => ({ ...state, ...defaultState, ...data })),
  }))
);
