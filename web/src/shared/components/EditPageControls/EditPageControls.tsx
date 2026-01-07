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
      <Button
        className="cancel"
        {...{
          text: m.controls_cancel(),
          variant: 'secondary',
          ...cancelProps,
        }}
      />
      <Button
        {...{
          text: m.controls_submit(),
          variant: 'primary',
          ...submitProps,
        }}
      />
    </div>
  );
};
