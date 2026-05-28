import { createFileRoute } from '@tanstack/react-router';
import { UsersOverviewPage } from '../../../pages/UsersOverviewPage/UsersOverviewPage';

export const Route = createFileRoute('/_authorized/_default/users')({
  component: UsersOverviewPage,
});
