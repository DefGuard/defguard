import { createFileRoute, redirect } from '@tanstack/react-router';
import { queryClient } from '../app/query';
import { useAuth } from '../shared/hooks/useAuth';
import { userMeQueryOptions } from '../shared/query';

export const Route = createFileRoute('/')({
  component: RouteComponent,
  loader: async () => {
    const responseData = await queryClient.fetchQuery(userMeQueryOptions).catch(() => {
      throw redirect({ to: '/auth/login' });
    });

    if (responseData.data) {
      useAuth.getState().setUser(responseData.data);
    }
  },
});

function RouteComponent() {
  return null;
}
