import './style.scss';
import type { PropsWithChildren } from 'react';
import { LayoutGrid } from '../../components/LayoutGrid/LayoutGrid';

export const TablePageLayout = ({ children }: PropsWithChildren) => {
  return <LayoutGrid className="table-page-layout">{children}</LayoutGrid>;
};
