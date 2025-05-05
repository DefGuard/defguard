import { useMemo } from 'react';
import { FieldValues, useController, UseControllerProps } from 'react-hook-form';

import { DialogSelect } from './DialogSelect';
import { DialogSelectProps } from './types';

type Props<T extends FieldValues, B, I extends number | string> = {
  controller: UseControllerProps<T>;
  forceShowErrorMessage?: boolean;
  onChange?: () => void;
} & Omit<DialogSelectProps<B, I>, 'selected' | 'errorMessage'>;

export const FormDialogSelect = <
  T extends FieldValues,
  B extends object,
  I extends number | string,
>({
  controller,
  onChange: onChangeExternal,
  forceShowErrorMessage = false,
  ...selectProps
}: Props<T, B, I>) => {
  const {
    field: { value, onChange },
    fieldState: { error, isDirty, isTouched },
    formState: { isSubmitted },
  } = useController(controller);

  const errorMessage = useMemo(() => {
    if (
      (error && (isDirty || isTouched)) ||
      (!error && isSubmitted) ||
      forceShowErrorMessage
    ) {
      return error?.message;
    }
    return undefined;
  }, [error, forceShowErrorMessage, isDirty, isSubmitted, isTouched]);

  return (
    <DialogSelect
      {...selectProps}
      onChange={(selected) => {
        onChange(selected);
        onChangeExternal?.();
      }}
      selected={value}
      errorMessage={errorMessage}
    />
  );
};
