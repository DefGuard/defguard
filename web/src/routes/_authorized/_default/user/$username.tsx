import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { UserProfileTab } from '../../../../pages/user-profile/UserProfilePage/tabs/types';
import { UserProfilePage } from '../../../../pages/user-profile/UserProfilePage/UserProfilePage';
import {
  getUserApiTokensQueryOptions,
  getUserAuthKeysQueryOptions,
  userProfileQueryOptions,
} from '../../../../shared/query';

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

export const Route = createFileRoute('/_authorized/_default/user/$username')({
  component: UserProfilePage,
  validateSearch: searchSchema,
  loader: ({ params, context: { queryClient } }) => {
    return Promise.all([
      queryClient.ensureQueryData(userProfileQueryOptions(params.username)),
      queryClient.ensureQueryData(getUserAuthKeysQueryOptions(params.username)),
      queryClient.ensureQueryData(getUserApiTokensQueryOptions(params.username)),
    ]);
  },
});
