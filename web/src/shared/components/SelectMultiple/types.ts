import type { IconKindValue } from '../../defguard-ui/components/Icon';
import type {
  SelectionKey,
  SelectionOption,
  SelectionSectionCustomRender,
  SelectionSectionProps,
} from '../SelectionSection/type';

type SelectionModalOverrides<T extends SelectionKey, M = unknown> = Pick<
  SelectionSectionProps<T, M>,
  'enableDividers' | 'itemGap' | 'searchPlaceholder' | 'visibleItemsLimit'
> & {
  contentClassName?: string;
};

export type SelectMultipleProps<T extends SelectionKey, M = unknown> = {
  options: SelectionOption<T>[];
  selected: Set<T>;
  modalTitle: string;
  editText: string;
  editIcon?: IconKindValue;
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
  selectionModalProps?: SelectionModalOverrides<T, M>;
};
