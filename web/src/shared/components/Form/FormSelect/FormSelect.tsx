import { useMemo } from 'react';
import {
  FieldValues,
  useController,
  UseControllerProps,
} from 'react-hook-form';

import { Select, SelectOption, SelectProps } from '../../layout/Select/Select';

interface Props<T extends FieldValues, Y>
  extends Omit<SelectProps<Y>, 'onChange'> {
  controller: UseControllerProps<T>;
}

export const FormSelect = <T extends FieldValues, Y>({
  controller,
  ...rest
}: Props<T, Y>) => {
  const {
    field,
    fieldState: { isDirty, isTouched, error },
  } = useController(controller);

  const isInvalid = useMemo(
    () => error && (isDirty || isTouched),
    [error, isDirty, isTouched]
  );

  const isValid = useMemo(
    () => !isInvalid && (isTouched || isDirty),
    [isDirty, isInvalid, isTouched]
  );

  return (
    <Select
      {...rest}
      selected={field.value as SelectOption<Y> | SelectOption<Y>[]}
      valid={isValid}
      invalid={isInvalid}
      errorMessage={error?.message}
      onChange={(res) => field.onChange(res)}
      inForm={true}
    />
  );
};
