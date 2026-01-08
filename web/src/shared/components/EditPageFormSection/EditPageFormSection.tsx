import type { PropsWithChildren } from 'react';
import './style.scss';

type Props = {
  label: string;
} & PropsWithChildren;

export const EditPageFormSection = ({ label, children }: Props) => {
  return (
    <div className="edit-page-form-section">
      <p className="label">{label}</p>
      <div className="content">{children}</div>
    </div>
  );
};
