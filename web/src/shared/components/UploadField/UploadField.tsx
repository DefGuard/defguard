import clsx from 'clsx';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { getFile } from 'easy-file-picker';
import { m } from '../../../paraglide/messages';
import { Button } from '../../defguard-ui/components/Button/Button';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { Icon, IconKind } from '../../defguard-ui/components/Icon';
import { ThemeVariable } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import type { UploadFieldProps } from './types';

export const UploadField = ({
  value,
  className,
  error,
  id,
  acceptedExtensions,
  loading,
  disabled,
  testId,
  title,
  onChange,
}: UploadFieldProps) => {
  const valuePresent = isPresent(value);
  const { mutate, isPending } = useMutation({
    mutationFn: getFile,
    onSuccess: (result) => {
      onChange?.(result);
    },
  });

  return (
    <div data-testid={testId} className={clsx('upload-field', className)} id={id}>
      <div className="inner-track">
        {valuePresent && (
          <div className="file-row">
            <Icon staticColor={ThemeVariable.FgAction} icon={IconKind.Config} size={20} />
            <p>{value.name}</p>
            <button
              className="delete"
              type="button"
              onClick={() => {
                onChange?.(null);
              }}
            >
              <Icon icon={IconKind.Delete} size={20} />
            </button>
          </div>
        )}
        {!valuePresent && (
          <Button
            iconLeft="upload"
            variant="outlined"
            text={title ?? m.cmp_file_upload_button()}
            loading={loading || isPending}
            disabled={disabled}
            onClick={() => {
              mutate({
                acceptedExtensions,
              });
            }}
          />
        )}
      </div>
      <FieldError error={error} />
    </div>
  );
};
