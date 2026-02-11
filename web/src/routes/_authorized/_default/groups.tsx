import { createFileRoute } from '@tanstack/react-router';
import { GroupsPage } from '../../../pages/GroupsPage/GroupsPage';
import {
  getGroupsInfoQueryOptions,
  getUsersOverviewQueryOptions,
} from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/groups')({
  component: GroupsPage,
  loader: ({ context }) => {
    return Promise.all([
      context.queryClient.ensureQueryData(getGroupsInfoQueryOptions),
      context.queryClient.ensureQueryData(getUsersOverviewQueryOptions),
    ]);
  },
});
