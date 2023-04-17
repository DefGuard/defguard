import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { FieldValues, useController, UseControllerProps } from 'react-hook-form';

import { Input, InputProps } from '../../layout/Input/Input';

interface Props<T extends FieldValues> extends InputProps {
  controller: UseControllerProps<T>;
}
export const FormInput = <T extends FieldValues>({ controller, ...rest }: Props<T>) => {
  const {
    field,
    fieldState: { isDirty, isTouched, error },
    formState: { isSubmitted },
  } = useController(controller);
  const isInvalid = useMemo(() => {
    if (
      (!isUndefined(error) && (isDirty || isTouched)) ||
      (!isUndefined(error) && isSubmitted)
    ) {
      return true;
    }
    return false;
  }, [error, isDirty, isSubmitted, isTouched]);
  const isValid = useMemo(
    () => !isInvalid && (isTouched || isDirty || isSubmitted),
    [isDirty, isInvalid, isSubmitted, isTouched]
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
