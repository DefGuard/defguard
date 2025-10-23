import { createFileRoute } from '@tanstack/react-router';
import { queryClient } from '../../../app/query';
import { UserProfilePage } from '../../../pages/user-profile/UserProfilePage/UserProfilePage';
import { userProfileQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/user/$username')({
  component: UserProfilePage,
  loader: ({ params }) => {
    return queryClient.ensureQueryData(userProfileQueryOptions(params.username));
  },
});
