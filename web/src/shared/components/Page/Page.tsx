import clsx from 'clsx';
import type { HtmlHTMLAttributes, PropsWithChildren, Ref } from 'react';
import { PageTopBar } from '../PageTopBar/PageTopBar';

interface Props extends HtmlHTMLAttributes<HTMLDivElement>, PropsWithChildren {
  ref?: Ref<HTMLDivElement>;
  title: string;
}

export const Page = ({ title, children, className, ...containerProps }: Props) => {
  return (
    <div className={clsx('page', className)} {...containerProps}>
      <PageTopBar title={title} />
      <div className="page-content">{children}</div>
    </div>
  );
};
