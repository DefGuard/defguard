import { createFileRoute, redirect } from '@tanstack/react-router';
import z from 'zod';
import api from '../../shared/api/api';
import { LoginPage } from '../../shared/components/LoginPage/LoginPage';
import { LoaderSpinner } from '../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';
import { useAuth } from '../../shared/hooks/useAuth';

const searchSchema = z.object({
  code: z.string(),
  state: z.string(),
});

// This is used when someone wants to login through a provider
export const Route = createFileRoute('/auth/callback')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps }) => {
    try {
      const search = deps.search;
      const response = await api.openid.callback(search);
      useAuth.getState().authSubject.next(response.data);
    } catch (_) {
      throw redirect({ to: '/auth/login', replace: true });
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <LoginPage>
      <LoaderSpinner size={64} />
    </LoginPage>
  );
}
