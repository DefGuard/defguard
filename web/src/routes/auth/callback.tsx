import { createFileRoute, redirect } from '@tanstack/react-router';
import z from 'zod';
import { LoginLoadingPage } from '../../pages/auth/LoginLoading/LoginLoadingPage';
import api from '../../shared/api/api';
import { getAuthErrorFromException } from '../../shared/api/apiErrorMessages';
import { useAuth } from '../../shared/hooks/useAuth';

const searchSchema = z.union([
  z.object({
    code: z.string(),
    state: z.string(),
  }),
  z.object({
    error: z.string(),
    error_description: z.string().optional(),
    state: z.string().optional(),
  }),
]);

// This is used when someone wants to login through a provider
export const Route = createFileRoute('/auth/callback')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps, context }) => {
    const search = deps.search;
    if ('error' in search) {
      throw redirect({
        to: '/auth/login',
        replace: true,
        search: { authErrorDetail: search.error_description ?? search.error },
      });
    }
    try {
      const response = await api.openid.callback(search);
      setTimeout(() => {
        void context.queryClient.invalidateQueries({
          queryKey: ['me'],
        });
        void context.queryClient.invalidateQueries({
          queryKey: ['session-info'],
        });
        useAuth.getState().authSubject.next(response.data);
      }, 1000);
    } catch (e) {
      throw redirect({
        to: '/auth/login',
        replace: true,
        search: getAuthErrorFromException(e),
      });
    }
  },
  component: LoginLoadingPage,
});
