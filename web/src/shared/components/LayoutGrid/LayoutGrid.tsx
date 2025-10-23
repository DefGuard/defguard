import clsx from 'clsx';
import './style.scss';
import type { HtmlHTMLAttributes, PropsWithChildren } from 'react';

export const LayoutGrid = ({
  children,
  className,
  ...props
}: PropsWithChildren & HtmlHTMLAttributes<HTMLDivElement>) => {
  return (
    <div className={clsx('layout-grid', className)} {...props}>
      {children}
    </div>
  );
};
