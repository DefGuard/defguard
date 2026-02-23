import { m } from '../../../paraglide/messages';
import { Button } from '../../defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../defguard-ui/components/Button/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import './style.scss';

type Props = {
  cancelProps?: Partial<ButtonProps>;
  submitProps?: Partial<ButtonProps>;
  deleteProps?: ButtonProps;
};

export const EditPageControls = ({ cancelProps, deleteProps, submitProps }: Props) => {
  return (
    <div className="edit-page-controls">
      {isPresent(deleteProps) && (
        <Button
          {...{
            variant: 'critical',
            ...deleteProps,
          }}
        />
      )}
      <div className="edit-page-controls__actions">
        {isPresent(cancelProps) && (
          <Button
            className="cancel"
            {...{
              text: m.controls_cancel(),
              variant: 'secondary',
              ...cancelProps,
            }}
          />
        )}
        <Button
          {...{
            text: m.controls_save_changes(),
            variant: 'primary',
            ...submitProps,
          }}
        />
      </div>
    </div>
  );
};
