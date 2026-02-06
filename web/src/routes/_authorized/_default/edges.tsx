import { createFileRoute } from '@tanstack/react-router';
import { EdgesPage } from '../../../pages/EdgesPage/EdgesPage';
import { getEdgesQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/edges')({
  component: EdgesPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getEdgesQueryOptions);
  },
});
