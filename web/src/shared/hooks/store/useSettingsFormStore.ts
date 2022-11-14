import { Subject } from 'rxjs';
import create from 'zustand';

export interface SettingsFormStore {
  editMode: boolean;
  submitSubject: Subject<void>;
  loading: boolean;
  setState: (state: Partial<SettingsFormStore>) => void;
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export const useSettingsFormStore = create<SettingsFormStore>((set) => ({
  editMode: false,
  loading: false,
  submitSubject: new Subject<void>(),
  setState: (newState) => set((oldState) => ({ ...oldState, ...newState })),
}));
