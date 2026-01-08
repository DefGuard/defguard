import type { HtmlHTMLAttributes, PropsWithChildren } from 'react';
import './style.scss';
import clsx from 'clsx';

export const ProfileCard = ({
  children,
  className,
  ...props
}: PropsWithChildren & HtmlHTMLAttributes<HTMLDivElement>) => {
  return (
    <div className={clsx('profile-card', className)} {...props}>
      {children}
    </div>
  );
};
