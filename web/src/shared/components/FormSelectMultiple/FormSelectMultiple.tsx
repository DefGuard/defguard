import { useMemo } from 'react';
import { useFormFieldError } from '../../defguard-ui/hooks/useFormFieldError';
import { useFieldContext } from '../../form';
import type { SelectionKey } from '../SelectionSection/type';
import { SelectMultiple } from '../SelectMultiple/SelectMultiple';
import type { SelectMultipleProps } from '../SelectMultiple/types';

type Props<T extends SelectionKey> = Omit<
  SelectMultipleProps<T>,
  'selected' | 'error' | 'onSelectionChange'
> & {
  onSelectionChange?: (v: T[]) => void;
};

export const FormSelectMultiple = <T extends SelectionKey>({
  onSelectionChange,
  ...props
}: Props<T>) => {
  const field = useFieldContext<T[]>();
  const error = useFormFieldError();

  const selected = useMemo(() => new Set(field.state.value), [field.state.value]);

  return (
    <SelectMultiple
      {...props}
      error={error}
      selected={selected}
      onSelectionChange={(val) => {
        field.handleChange(val);
        onSelectionChange?.(val);
      }}
    />
  );
};
