import { ReactNode } from 'react';

export type DialogSelectProps<T, I> = {
  options: T[];
  identKey: keyof T;
  selected: I[];
  renderTagContent: (value: T) => ReactNode;
  // if not provided will use renderTagContent instead
  renderDialogListItem?: (value: T) => ReactNode;
  errorMessage?: string;
  label?: string;
  // Can replace searchFn, when given only keys it will use util searchByKeys, searchFn prop takes priority if both given.
  searchKeys?: Array<keyof T>;
  disabled?: boolean;
  searchFn?: DialogSelectSearch<T>;
  onChange?: (values: I[]) => void;
};

export type DialogSelectSearch<T> = (obj: T, searchedValue: string) => boolean;
