import { useState } from 'react';
import { Input } from '../../defguard-ui/components/Input/Input';
import type { FormInputProps } from '../../defguard-ui/components/Input/types';
import { useFormFieldError } from '../../defguard-ui/hooks/useFormFieldError';
import { useFieldContext } from '../../form-context';

type Props = Pick<FormInputProps, 'label' | 'helper' | 'required'> & {
  stored: boolean;
};

export const FormSecretInput = ({ stored, required, ...props }: Props) => {
  const field = useFieldContext<string | number | null | undefined>();
  const error = useFormFieldError();
  const [focused, setFocused] = useState(false);
  const masked = stored && !focused && field.state.value === undefined;

  return (
    <Input
      {...props}
      testId={`field-${field.name}`}
      type="password"
      required={required && !stored}
      value={field.state.value ?? null}
      placeholder={masked ? '•••••••••••••••••' : undefined}
      error={error}
      onChange={field.handleChange}
      onFocus={() => setFocused(true)}
      onBlur={() => {
        setFocused(false);
        if (stored && !field.state.value) {
          field.handleChange(undefined);
        }
        field.handleBlur();
      }}
    />
  );
};
