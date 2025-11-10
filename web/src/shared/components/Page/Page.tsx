import './style.scss';
import { useMatch } from '@tanstack/react-router';
import clsx from 'clsx';
import type { HtmlHTMLAttributes, PropsWithChildren, Ref } from 'react';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { useApp } from '../../hooks/useApp';
import { PageTopBar } from '../PageTopBar/PageTopBar';

interface Props extends HtmlHTMLAttributes<HTMLDivElement>, PropsWithChildren {
  ref?: Ref<HTMLDivElement>;
  title: string;
}

export const Page = ({ title, children, className, ...containerProps }: Props) => {
  const isMatched = useMatch({ from: '/_authorized' });
  const isNavOpen = useApp((s) => s.navigationOpen && isPresent(isMatched));

  return (
    <div
      className={clsx('page', className, {
        nav: isNavOpen,
      })}
      {...containerProps}
    >
      <PageTopBar title={title} navOpen={isNavOpen} />
      <div className="page-content">{children}</div>
    </div>
  );
};
