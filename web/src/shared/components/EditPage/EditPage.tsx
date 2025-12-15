import type { HTMLProps, PropsWithChildren } from 'react';
import { LayoutGrid } from '../LayoutGrid/LayoutGrid';
import { Page } from '../Page/Page';
import './style.scss';
import clsx from 'clsx';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { Breadcrumbs } from '../Breadcrumbs/Breadcrumbs';
import type { BreadcrumbsProps } from '../Breadcrumbs/types';

type Props = {
  pageTitle: string;
  formTitle: string;
  links?: BreadcrumbsProps['links'];
} & PropsWithChildren &
  HTMLProps<HTMLDivElement>;

export const EditPage = ({
  formTitle,
  pageTitle,
  children,
  className,
  links,
  ...containerProps
}: Props) => {
  return (
    <Page title={pageTitle} className={clsx('edit-page', className)} {...containerProps}>
      {isPresent(links) && <Breadcrumbs links={links} />}
      <LayoutGrid>
        <div className="main-content">
          <p>{formTitle}</p>
          <div className="card">{children}</div>
        </div>
      </LayoutGrid>
    </Page>
  );
};
