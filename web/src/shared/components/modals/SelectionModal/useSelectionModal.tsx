import { create } from 'zustand';
import type { SelectionKey, SelectionOption } from '../../SelectionSection/type';

interface StoreValues {
  title: string;
  options: SelectionOption<SelectionKey>[];
  selected: Set<number> | Set<string>;
  isOpen: boolean;
  onSubmit?: (values: Array<SelectionKey>) => void;
}

const defaultValues: StoreValues = {
  title: 'Select',
  options: [],
  selected: new Set<number>(),
  isOpen: false,
};

interface Store extends StoreValues {
  reset: () => void;
}

export const useSelectionModal = create<Store>((set) => ({
  ...defaultValues,
  reset: () => set(defaultValues),
}));
