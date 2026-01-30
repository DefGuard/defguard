import { createFileRoute } from '@tanstack/react-router';
import { EdgeListPage } from '../../../../pages/EdgeListPage/EdgeListPage';

export const Route = createFileRoute('/_authorized/_default/edge/')({
  component: EdgeListPage,
});
