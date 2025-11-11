export type SelectionSectionKey = string | number;

export type SelectionSectionOption<T> = {
  id: T;
  label: string;
  meta?: unknown;
  // if there is a need to search in more then label itself
  searchFields?: string[];
};

export interface SelectionSectionProps<T extends SelectionSectionKey> {
  selection: Set<T>;
  onChange: (value: Set<T>) => void;
  options: SelectionSectionOption<T>[];
  itemHeight?: number;
  itemGap?: number;
  className?: string;
  id?: string;
}
