import './style.scss';

import clsx from 'clsx';
import type { PropsWithChildren } from 'react';

import { PageContainer } from '../PageContainer/PageContainer';

type Props = {
  id: string;
  className?: string;
  withDefaultPadding?: boolean;
} & PropsWithChildren;

export const PageLayout = ({
  id,
  className,
  children,
  withDefaultPadding = false,
}: Props) => {
  return (
    <PageContainer
      id={id}
      className={clsx('page-layout', 'standard', className)}
      withDefaultPadding={withDefaultPadding}
    >
      {children}
    </PageContainer>
  );
};
