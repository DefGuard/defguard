import './style.scss';

import clsx from 'clsx';
import { PropsWithChildren } from 'react';

import { PageContainer } from '../PageContainer/PageContainer';

type Props = {
  id: string;
  className?: string;
} & PropsWithChildren;

export const PageLayout = ({ id, className, children }: Props) => {
  return (
    <PageContainer id={id} className={clsx('page-layout', 'standard', className)}>
      {children}
    </PageContainer>
  );
};
