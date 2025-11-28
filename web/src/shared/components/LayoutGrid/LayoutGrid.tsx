import clsx from 'clsx';
import './style.scss';
import type { HtmlHTMLAttributes, PropsWithChildren } from 'react';

type Props = {
  variant?: 'default' | 'wizard';
} & PropsWithChildren &
  HtmlHTMLAttributes<HTMLDivElement>;

export const LayoutGrid = ({
  children,
  className,
  variant = 'default',
  ...props
}: Props) => {
  return (
    <div className={clsx('layout-grid', className, `variant-${variant}`)} {...props}>
      {children}
    </div>
  );
};
