import type { HTMLProps, PropsWithChildren } from 'react';
import { LayoutGrid } from '../LayoutGrid/LayoutGrid';
import { Page } from '../Page/Page';
import './style.scss';
import clsx from 'clsx';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { Breadcrumbs } from '../Breadcrumbs/Breadcrumbs';
import type { BreadcrumbsProps } from '../Breadcrumbs/types';
import { EditHeader } from '../EditHeader/EditHeader';
import type { EditHeaderProps } from '../EditHeader/types';

type Props = {
  pageTitle: string;
  headerProps: EditHeaderProps;
  links?: BreadcrumbsProps['links'];
} & PropsWithChildren &
  HTMLProps<HTMLDivElement>;

export const EditPage = ({
  pageTitle,
  children,
  className,
  links,
  headerProps,
  ...containerProps
}: Props) => {
  return (
    <Page title={pageTitle} className={clsx('edit-page', className)} {...containerProps}>
      {isPresent(links) && <Breadcrumbs links={links} />}
      <LayoutGrid>
        <div className="main-content">
          <EditHeader {...headerProps} />
          <div className="card">{children}</div>
        </div>
      </LayoutGrid>
    </Page>
  );
};
