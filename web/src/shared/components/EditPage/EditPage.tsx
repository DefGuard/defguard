import type { HTMLProps, PropsWithChildren } from 'react';
import { LayoutGrid } from '../LayoutGrid/LayoutGrid';
import { Page } from '../Page/Page';
import './style.scss';
import clsx from 'clsx';

type Props = {
  pageTitle: string;
  formTitle: string;
} & PropsWithChildren &
  HTMLProps<HTMLDivElement>;

export const EditPage = ({
  formTitle,
  pageTitle,
  children,
  className,
  ...containerProps
}: Props) => {
  return (
    <Page title={pageTitle} className={clsx('edit-page', className)} {...containerProps}>
      <LayoutGrid>
        <div className="main-content">
          <p>{formTitle}</p>
          <div className="card">{children}</div>
        </div>
      </LayoutGrid>
    </Page>
  );
};
