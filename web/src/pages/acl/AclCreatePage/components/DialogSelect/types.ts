import { ReactNode } from 'react';

export type DialogSelectProps<T, I> = {
  options: T[];
  identKey: keyof T;
  selected: I[];
  renderTagContent: (value: T) => ReactNode;
  errorMessage?: string;
  label?: string;
  onChange?: (values: I[]) => void;
};
