import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { queryClient } from '../../../app/query';
import { UserProfileTab } from '../../../pages/user-profile/UserProfilePage/tabs/types';
import { UserProfilePage } from '../../../pages/user-profile/UserProfilePage/UserProfilePage';
import {
  getUserApiTokensQueryOptions,
  getUserAuthKeysQueryOptions,
  userProfileQueryOptions,
} from '../../../shared/query';

const searchSchema = z.object({
  tab: z
    .enum([
      UserProfileTab.Details,
      UserProfileTab.Devices,
      UserProfileTab.AuthKeys,
      UserProfileTab.ApiTokens,
    ])
    .default(UserProfileTab.Details),
});

export const Route = createFileRoute('/_authorized/user/$username')({
  component: UserProfilePage,
  validateSearch: searchSchema,
  loader: ({ params }) => {
    return Promise.all([
      queryClient.ensureQueryData(userProfileQueryOptions(params.username)),
      queryClient.ensureQueryData(getUserAuthKeysQueryOptions(params.username)),
      queryClient.ensureQueryData(getUserApiTokensQueryOptions(params.username)),
    ]);
  },
});
