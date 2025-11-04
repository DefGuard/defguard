import { createFileRoute } from '@tanstack/react-router';
import { UsersOverviewPage } from '../../pages/UsersOverviewPage/UsersOverviewPage';
import { getUsersQueryOptions } from '../../shared/query';

export const Route = createFileRoute('/_authorized/users')({
  component: UsersOverviewPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getUsersQueryOptions);
  },
});
