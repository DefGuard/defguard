import { useMemo } from 'react';
import {
  FieldValues,
  useController,
  UseControllerProps,
} from 'react-hook-form';

import { Input, InputProps } from '../../layout/Input/Input';

interface Props<T extends FieldValues> extends InputProps {
  controller: UseControllerProps<T>;
}
export const FormInput = <T extends FieldValues>({
  controller,
  ...rest
}: Props<T>) => {
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
    <Input
      {...rest}
      {...field}
      invalid={isInvalid}
      valid={isValid}
      errorMessage={error?.message}
    />
  );
};
