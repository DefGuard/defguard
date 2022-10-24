import { BehaviorSubject, Subject } from 'rxjs';
import create from 'zustand';
import { devtools } from 'zustand/middleware';

import { WizardStore } from '../types/interfaces';
import { FormStatus, Location, User, WizardNetwork } from '../types/types';

const defaultState = {
  stepsCount: 2,
  network: new BehaviorSubject({} as WizardNetwork),
  users: [],
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
  devtools((set, get) => ({
    stepsCount: 2,
    network: new BehaviorSubject({} as WizardNetwork),
    users: [],
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
    addUser: (user: User) =>
      set((state) => ({ users: [...state.users, user] })),
    removeUser: (user: User) =>
      set((state) => ({
        users: state.users.filter((u) => u.email !== user.email),
      })),
    setNetwork: (network: WizardNetwork) => {
      const networkObserver = get().network;
      networkObserver.next({ ...networkObserver.getValue(), ...network });
    },
    setFormStatus: (formStatus: FormStatus) =>
      set((state) => ({
        formStatus: { ...state.formStatus, ...formStatus },
      })),
    setState: (data) => set((state) => ({ ...state, ...data })),
    resetStore: (data) =>
      set((state) => ({ ...state, ...defaultState, ...data })),
  }))
);
