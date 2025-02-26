import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { FieldValues, useController, UseControllerProps } from 'react-hook-form';

import { DateInput } from './DateInput';

type Props<T extends FieldValues> = {
  onChange?: (value: string | null) => void;
  controller: UseControllerProps<T>;
  label?: string;
};

export const FormDateInput = <T extends FieldValues>({
  onChange,
  controller,
  label,
}: Props<T>) => {
  const {
    field: { value, onChange: fieldChange },
    fieldState: { isDirty, isTouched, error },
    formState: { isSubmitted },
  } = useController(controller);

  const errorMessage = useMemo(() => {
    if (
      (!isUndefined(error) && (isDirty || isTouched)) ||
      (!isUndefined(error) && isSubmitted)
    ) {
      return error.message;
    }
    return undefined;
  }, [error, isDirty, isSubmitted, isTouched]);

  return (
    <DateInput
      selected={value}
      onChange={(val) => {
        fieldChange(val);
        onChange?.(val);
      }}
      label={label}
      errorMessage={errorMessage}
    />
  );
};
