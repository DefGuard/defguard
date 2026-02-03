import { useSuspenseQuery } from '@tanstack/react-query';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getEdgesQueryOptions } from '../../shared/query';
import { EdgesTable } from './EdgesTable';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';

export const EdgesPage = () => {
  const { data: edges } = useSuspenseQuery(getEdgesQueryOptions);
  return (
    <Page title={m.edge_title()}>
      <TablePageLayout>
        {isPresent(edges) && <EdgesTable edges={edges} />}
      </TablePageLayout>
    </Page>
  );
};
