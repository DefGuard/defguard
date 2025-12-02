import type {
  SelectionSectionKey,
  SelectionSectionOption,
} from '../SelectionSection/type';

export type SelectMultipleProps<T extends SelectionSectionKey> = {
  options: SelectionSectionOption<T>[];
  selected: Set<T>;
  modalTitle: string;
  editText: string;
  toggleText: string;
  counterText: (count: number) => string;
  onChange: (value: Array<T>) => void;
};
