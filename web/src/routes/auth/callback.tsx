import { createFileRoute, redirect } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import z from 'zod';
import { LoginLoadingPage } from '../../pages/auth/LoginLoading/LoginLoadingPage';
import api from '../../shared/api/api';
import { type ApiError, WebErrorCode } from '../../shared/api/types';
import { useAuth } from '../../shared/hooks/useAuth';

const getAuthErrorFromException = (e: unknown): WebErrorCode | undefined => {
  const code = (e as AxiosError<ApiError>).response?.data?.code;
  if (code === WebErrorCode.UserGroupsNotSynced || code === WebErrorCode.LicenseLimitReached) {
    return code;
  }
  return undefined;
};

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
        search: { authError: search.error_description ?? search.error },
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
        search: { authError: getAuthErrorFromException(e) },
      });
    }
  },
  component: LoginLoadingPage,
});
