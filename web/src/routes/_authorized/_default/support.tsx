import { createFileRoute } from '@tanstack/react-router';
import { SupportPage } from '../../../pages/SupportPage/SupportPage';

export const Route = createFileRoute('/_authorized/_default/support')({
  component: SupportPage,
});
