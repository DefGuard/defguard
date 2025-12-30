export type SelectionKey = string | number;

export type SelectionOption<T> = {
  id: T;
  label: string;
  meta?: unknown;
  // if there is a need to search in more then label itself
  searchFields?: string[];
};

export interface SelectionSectionProps<T extends SelectionKey> {
  selection: Set<T>;
  onChange: (value: Set<T>) => void;
  options: SelectionOption<T>[];
  itemHeight?: number;
  itemGap?: number;
  className?: string;
  id?: string;
}
