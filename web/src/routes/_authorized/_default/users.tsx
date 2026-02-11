import { createFileRoute } from '@tanstack/react-router';
import { UsersOverviewPage } from '../../../pages/UsersOverviewPage/UsersOverviewPage';
import { getUsersOverviewQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/users')({
  component: UsersOverviewPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getUsersOverviewQueryOptions);
  },
});
