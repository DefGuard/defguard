import type { SelectionKey, SelectionOption } from '../SelectionSection/type';

export type SelectMultipleProps<T extends SelectionKey> = {
  options: SelectionOption<T>[];
  selected: Set<T>;
  modalTitle: string;
  editText: string;
  toggleText: string;
  counterText: (count: number) => string;
  onChange: (value: Array<T>) => void;
};
