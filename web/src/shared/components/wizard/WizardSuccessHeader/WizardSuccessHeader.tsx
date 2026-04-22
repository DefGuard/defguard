import './style.scss';
import type { PropsWithChildren } from 'react';
import { isPresent } from '../../../defguard-ui/utils/isPresent';

interface Props extends PropsWithChildren {
  title: string;
}
export const WizardSuccessHeader = ({ title, children }: Props) => {
  return (
    <div className="wizard-success-header">
      <p className="title">{title}</p>
      {isPresent(children) && <div className="content">{children}</div>}
    </div>
  );
};
