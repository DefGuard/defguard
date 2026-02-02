import type {
  SelectionKey,
  SelectionOption,
  SelectionSectionCustomRender,
} from '../SelectionSection/type';

export type SelectMultipleProps<T extends SelectionKey, M = never> = {
  options: SelectionOption<T>[];
  selected: Set<T>;
  modalTitle: string;
  editText: string;
  toggleText?: string;
  error?: string;
  counterText: (count: number) => string;
  onChange: (value: Array<T>) => void;
  selectionCustomItemRender?: SelectionSectionCustomRender<T, M>;
};
