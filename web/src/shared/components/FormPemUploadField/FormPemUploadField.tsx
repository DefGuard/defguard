import { useState } from 'react';
import { m } from '../../../paraglide/messages';
import { useFormFieldError } from '../../defguard-ui/hooks/useFormFieldError';
import { useFieldContext } from '../../form-context';
import type { UploadFieldProps } from '../UploadField/types';
import { UploadField } from '../UploadField/UploadField';

type Props = Omit<UploadFieldProps, 'value' | 'onChange' | 'stored' | 'label'> & {
  label: string;
  pemMarker: string;
};

export const FormPemUploadField = ({ pemMarker, ...props }: Props) => {
  const field = useFieldContext<string | null | undefined>();
  const error = useFormFieldError();
  const [formatError, setFormatError] = useState<string>();

  return (
    <UploadField
      {...props}
      testId={`field-${field.name}`}
      error={props.error ?? formatError ?? error}
      stored={field.state.value !== null}
      onChange={async (file) => {
        const text = file ? await file.text() : null;
        const valid = text === null || text.includes(pemMarker);
        setFormatError(valid ? undefined : m.form_error_file_format());
        field.handleChange(valid ? text : null);
      }}
    />
  );
};
