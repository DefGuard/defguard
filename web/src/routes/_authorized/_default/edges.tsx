import { createFileRoute } from '@tanstack/react-router';
import { EdgesPage } from '../../../pages/EdgesPage/EdgesPage';

export const Route = createFileRoute('/_authorized/_default/edges')({
  component: EdgesPage,
});
