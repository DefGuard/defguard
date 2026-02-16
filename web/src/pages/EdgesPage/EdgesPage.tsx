import './styles.scss';
import { Suspense } from 'react';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { EdgesTable } from './EdgesTable';

export const EdgesPage = () => {
  return (
    <Page title={m.edge_title()} id="edges-page">
      <Suspense fallback={<TableSkeleton />}>
        <TablePageLayout>
          <EdgesTable />
        </TablePageLayout>
      </Suspense>
    </Page>
  );
};
