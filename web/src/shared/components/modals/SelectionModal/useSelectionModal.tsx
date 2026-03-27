import { create } from 'zustand';
import { m } from '../../../../paraglide/messages';
import type {
  SelectionKey,
  SelectionOption,
  SelectionSectionProps,
} from '../../SelectionSection/type';

type SectionProps = SelectionSectionProps<SelectionKey, unknown>;

interface StoreValues {
  title: string;
  options: SelectionOption<SelectionKey>[];
  selected: Set<number> | Set<string>;
  isOpen: boolean;
  itemGap: number;
  enableDividers: boolean;
  onSubmit?: (values: Array<SelectionKey>) => void;
  onCancel?: () => void;
  orderItems?: SectionProps['orderItems'];
  renderItem?: SectionProps['renderItem'];
}

const getDefaultValues = (): StoreValues => ({
  title: m.modal_selection_title(),
  options: [],
  selected: new Set<number>(),
  isOpen: false,
  itemGap: 8,
  enableDividers: false,
  onSubmit: undefined,
  orderItems: undefined,
  renderItem: undefined,
  onCancel: undefined,
});

interface Store extends StoreValues {
  reset: () => void;
}

export const useSelectionModal = create<Store>((set) => ({
  ...getDefaultValues(),
  reset: () => set(getDefaultValues()),
}));
