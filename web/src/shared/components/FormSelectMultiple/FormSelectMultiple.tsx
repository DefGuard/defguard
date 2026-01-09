import { useMemo } from 'react';
import { useFieldContext } from '../../form';
import type { SelectionKey } from '../SelectionSection/type';
import { SelectMultiple } from '../SelectMultiple/SelectMultiple';
import type { SelectMultipleProps } from '../SelectMultiple/types';

type Props<T extends SelectionKey> = Omit<
  SelectMultipleProps<T>,
  'selected' | 'onChange'
>;

export const FormSelectMultiple = <T extends SelectionKey>(props: Props<T>) => {
  const field = useFieldContext<SelectionKey[]>();

  const selected = useMemo(() => new Set(field.state.value), [field.state.value]);

  return <SelectMultiple {...props} selected={selected} onChange={field.handleChange} />;
};
