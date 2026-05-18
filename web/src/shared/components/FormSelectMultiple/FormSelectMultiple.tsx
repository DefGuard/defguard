import { useMemo } from 'react';
import { useFormFieldError } from '../../defguard-ui/hooks/useFormFieldError';
import { useFieldContext } from '../../form-context';
import type { SelectionKey } from '../SelectionSection/type';
import { SelectMultiple } from '../SelectMultiple/SelectMultiple';
import type { SelectMultipleProps } from '../SelectMultiple/types';

type Props<T extends SelectionKey, M = unknown> = Omit<
  SelectMultipleProps<T, M>,
  'selected' | 'error' | 'onSelectionChange'
> & {
  onSelectionChange?: (v: T[]) => void;
};

export const FormSelectMultiple = <T extends SelectionKey, M = unknown>({
  onSelectionChange,
  ...props
}: Props<T, M>) => {
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
