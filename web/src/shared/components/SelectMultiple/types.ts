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
  toggleValue: boolean;
  toggleText?: string;
  error?: string;
  counterText: (count: number) => string;
  onSelectionChange: (
    value: Array<T>,
    toggleValue?: boolean,
    onToggleChange?: (v: boolean) => void,
  ) => void;
  onToggleChange: (value: boolean) => void;
  selectionCustomItemRender?: SelectionSectionCustomRender<T, M>;
};
