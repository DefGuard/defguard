import type { HTMLProps } from 'react';
import './style.scss';
import clsx from 'clsx';

type Props = HTMLProps<HTMLDivElement>;

export const WizardCard = ({ className, children, ...props }: Props) => {
  return (
    <div className={clsx('wizard-card', className)} {...props}>
      {children}
    </div>
  );
};
