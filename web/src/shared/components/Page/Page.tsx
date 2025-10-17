import './style.scss';
import clsx from 'clsx';
import type { HtmlHTMLAttributes, PropsWithChildren, Ref } from 'react';

interface Props extends HtmlHTMLAttributes<HTMLDivElement>, PropsWithChildren {
  ref?: Ref<HTMLDivElement>;
}

export const Page = ({ children, className, ...containerProps }: Props) => {
  return (
    <div className={clsx('page', className)} {...containerProps}>
      <div className="page-content">{children}</div>
    </div>
  );
};
