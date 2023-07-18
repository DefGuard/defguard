import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { FieldValues, useController, UseControllerProps } from 'react-hook-form';

import {
  Select,
  SelectOption,
  SelectProps,
  SelectValue,
} from '../../layout/Select/Select';

interface Props<T extends FieldValues, Y extends SelectValue>
  extends Omit<SelectProps<Y>, 'onChange'> {
  controller: UseControllerProps<T>;
}

export const FormSelect = <T extends FieldValues, Y extends SelectValue>({
  controller,
  ...rest
}: Props<T, Y>) => {
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
    [isDirty, isInvalid, isSubmitted, isTouched],
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
