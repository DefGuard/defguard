import { createFileRoute, Outlet, redirect } from '@tanstack/react-router';
import { Navigation } from '../../shared/components/Navigation/Navigation';
import { getSessionInfoQueryOptions, getUserMeQueryOptions } from '../../shared/query';

export const Route = createFileRoute('/_authorized/_default')({
  beforeLoad: async ({ context, location }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;

    if (sessionInfo.is_admin) {
      return;
    }

    const me = (await context.queryClient.fetchQuery(getUserMeQueryOptions)).data;

    if (location.pathname !== `/user/${me.username}`) {
      throw redirect({
        to: '/user/$username',
        params: {
          username: me.username,
        },
        replace: true,
      });
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <>
      <Outlet />
      <Navigation />
    </>
  );
}
