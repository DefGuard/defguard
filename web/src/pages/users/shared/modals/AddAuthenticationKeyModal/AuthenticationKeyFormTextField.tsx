import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { FieldValues, useController, UseControllerProps } from 'react-hook-form';

import { InputFloatingErrors } from '../../../../../shared/defguard-ui/components/Layout/Input/types';
import { AuthenticationKeyTextField, TextareaProps } from './AuthenticationKeyTextField';

interface Props<T extends FieldValues>
  extends Omit<TextareaProps, 'floatingErrors' | 'onPaste'> {
  controller: UseControllerProps<T>;
  floatingErrors?: {
    title?: string;
    errorMessages?: string[];
  };
}
export const AuthenticationKeyFormTextField = <T extends FieldValues>({
  controller,
  floatingErrors,
  disabled,
  ...rest
}: Props<T>) => {
  const {
    field,
    fieldState: { isDirty, isTouched, error },
    formState: { isSubmitted },
  } = useController(controller);

  const isInvalid = useMemo(() => {
    if (disabled) return false;
    if (
      (!isUndefined(error) && (isDirty || isTouched)) ||
      (!isUndefined(error) && isSubmitted)
    ) {
      return true;
    }
    return false;
  }, [error, isDirty, isSubmitted, isTouched, disabled]);

  const floatingErrorsData = useMemo((): InputFloatingErrors | undefined => {
    if (floatingErrors && floatingErrors.title && error && error.types && isInvalid) {
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
  }, [error, floatingErrors, isInvalid]);

  return (
    <AuthenticationKeyTextField
      data-testid={`field-${controller.name}`}
      {...rest}
      {...field}
      invalid={isInvalid}
      errorMessage={error?.message}
      floatingErrors={floatingErrorsData}
      disabled={disabled}
    />
  );
};
