import { createFileRoute, redirect } from '@tanstack/react-router';
import { LoginLoadingPage } from '../../pages/auth/LoginLoading/LoginLoadingPage';
import {
  getSessionInfoQueryOptions,
  getUserMeQueryOptions,
} from '../../shared/query';

export const Route = createFileRoute('/auth/')({
  beforeLoad: async ({ context }) => {
    const sessionInfo = (
      await context.queryClient.ensureQueryData(getSessionInfoQueryOptions)
    ).data;

    if (sessionInfo.authorized) {
      if (sessionInfo.is_admin) {
        throw redirect({
          to: '/vpn-overview',
          replace: true,
        });
      }

      const me = (await context.queryClient.fetchQuery(getUserMeQueryOptions)).data;

      throw redirect({
        to: '/user/$username',
        params: {
          username: me.username,
        },
        replace: true,
      });
    }

    throw redirect({
      to: '/auth/login',
      replace: true,
    });
  },
  component: RouteComponent,
});

function RouteComponent() {
  return <LoginLoadingPage />;
}
