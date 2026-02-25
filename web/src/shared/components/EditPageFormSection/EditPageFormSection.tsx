import type { PropsWithChildren, ReactNode } from 'react';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import './style.scss';

type Props = {
  label: string;
  labelContent?: ReactNode;
} & PropsWithChildren;

export const EditPageFormSection = ({ label, children, labelContent }: Props) => {
  return (
    <div className="edit-page-form-section">
      {isPresent(labelContent) && (
        <div className="label-area">
          <p className="label">{label}</p>
          <p className="label-content">{labelContent}</p>
        </div>
      )}
      {!isPresent(labelContent) && <p className="label">{label}</p>}
      <div className="content">{children}</div>
    </div>
  );
};
