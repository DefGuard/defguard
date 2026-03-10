import { createFileRoute, redirect } from '@tanstack/react-router';
import { getSessionInfoQueryOptions, getUserMeQueryOptions } from '../shared/query';

export const Route = createFileRoute('/404')({
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (!sessionInfo.authorized) {
      throw redirect({ to: '/auth', replace: true });
    }
    if (sessionInfo.is_admin) {
      throw redirect({ to: '/vpn-overview', replace: true });
    }
    const me = (await context.queryClient.fetchQuery(getUserMeQueryOptions)).data;
    throw redirect({
      to: '/user/$username',
      params: {
        username: me.username,
      },
      replace: true,
    });
  },
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <div>
      <p>Not found!</p>
    </div>
  );
}
