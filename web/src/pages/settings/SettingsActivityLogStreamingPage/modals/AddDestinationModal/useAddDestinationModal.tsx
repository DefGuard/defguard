import { create } from 'zustand';

interface StoreValues {
  step: 'choice' | 'form';
  destination: 'logstash' | 'vector';
  isOpen: boolean;
}

const defaults: StoreValues = {
  isOpen: false,
  destination: 'logstash',
  step: 'choice',
};

interface Store extends StoreValues {
  reset: () => void;
}

export const useAddDestinationModal = create<Store>((set) => ({
  ...defaults,
  reset: () => set(defaults),
}));
