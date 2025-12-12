import { useFormFieldError } from '../../defguard-ui/hooks/useFormFieldError';
import { useFieldContext } from '../../form';
import type { UploadFieldProps } from '../UploadField/types';
import { UploadField } from '../UploadField/UploadField';

export const FormUploadField = (
  props: Omit<UploadFieldProps, 'value' | 'onChange' | 'error'>,
) => {
  const field = useFieldContext<File | null>();
  const error = useFormFieldError();

  return <UploadField error={error} value={field.state.value} {...props} />;
};
