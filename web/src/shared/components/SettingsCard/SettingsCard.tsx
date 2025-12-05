import type { HTMLProps } from 'react';
import './style.scss';
import clsx from 'clsx';

interface Props extends HTMLProps<HTMLDivElement> {}

export const SettingsCard = ({ className, children, ...props }: Props) => {
  return (
    <div className={clsx('settings-card', className)} {...props}>
      {children}
    </div>
  );
};
