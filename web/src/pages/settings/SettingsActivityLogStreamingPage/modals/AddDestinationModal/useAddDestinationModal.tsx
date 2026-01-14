import { create } from 'zustand';

interface StoreValues {
  step: 'choice' | 'destination';
  destination?: 'logstash' | 'vector';
  isOpen: boolean;
  name: string;
  url: string;
  username?: string;
  password?: string;
  certificate?: string;
}

const defaults: StoreValues = {
  isOpen: false,
  step: 'choice',
  name: '',
  url: '',
  username: '',
  password: '',
  certificate: '',
};

interface Store extends StoreValues {
  reset: () => void;
}

export const useAddDestinationModal = create<Store>((set) => ({
  ...defaults,
  reset: () => set(defaults),
}));
