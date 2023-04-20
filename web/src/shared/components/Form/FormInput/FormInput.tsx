import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { FieldValues, useController, UseControllerProps } from 'react-hook-form';

import { Input, InputFloatingErrors, InputProps } from '../../layout/Input/Input';

interface Props<T extends FieldValues> extends Omit<InputProps, 'floatingErrors'> {
  controller: UseControllerProps<T>;
  floatingErrors?: {
    title?: string;
    errorMessages?: string[];
  };
}
export const FormInput = <T extends FieldValues>({
  controller,
  floatingErrors,
  ...rest
}: Props<T>) => {
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

  const floatingErrorsData = useMemo((): InputFloatingErrors | undefined => {
    if (floatingErrors && floatingErrors.title && error && error.types && !isValid) {
      let errors: string[] = [];
      for (const val of Object.values(error.types)) {
        if (typeof val === 'string') {
          errors.push(val);
        }
        if (Array.isArray(val)) {
          errors = [...errors, ...val];
        }
      }
      if (floatingErrors.errorMessages && floatingErrors.errorMessages.length) {
        errors = [...errors, ...floatingErrors.errorMessages];
      }
      return {
        title: floatingErrors.title,
        errorMessages: errors,
      };
    }
    return undefined;
  }, [error, floatingErrors, isValid]);

  return (
    <Input
      {...rest}
      {...field}
      invalid={isInvalid}
      valid={isValid}
      errorMessage={error?.message}
      floatingErrors={floatingErrorsData}
    />
  );
};
