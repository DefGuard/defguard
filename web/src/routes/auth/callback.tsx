import { createFileRoute, redirect } from '@tanstack/react-router';
import z from 'zod';
import { LoginLoadingPage } from '../../pages/auth/LoginLoading/LoginLoadingPage';
import api from '../../shared/api/api';
import { useAuth } from '../../shared/hooks/useAuth';

const searchSchema = z.object({
  code: z.string(),
  state: z.string(),
});

// This is used when someone wants to login through a provider
export const Route = createFileRoute('/auth/callback')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps, context }) => {
    try {
      const search = deps.search;
      const response = await api.openid.callback(search);
      setTimeout(() => {
        void context.queryClient.invalidateQueries({
          queryKey: ['me'],
        });
        useAuth.getState().authSubject.next(response.data);
      }, 1000);
    } catch (_) {
      throw redirect({ to: '/auth/login', replace: true });
    }
  },
  component: LoginLoadingPage,
});
