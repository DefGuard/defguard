import { createFileRoute, redirect } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import z from 'zod';
import { OpenIdConsentPage } from '../pages/OpenIdConsentPage/OpenIdConsentPage';
import api from '../shared/api/api';
import { useAuth } from '../shared/hooks/useAuth';

const searchSchema = z.object({
  client_id: z.string().min(1),
  state: z.string().min(1),
  redirect_uri: z.string().min(1),
  response_type: z.string(),
  scope: z.string().min(1),
});

export const Route = createFileRoute('/consent')({
  validateSearch: searchSchema,
  loaderDeps: ({ search }) => ({ search }),
  loader: async ({ deps }) => {
    const clientResponse = await api.openIdClient
      .getOpenIdClient(deps.search.client_id)
      .catch((e: AxiosError) => {
        if (e.response?.status === 401) {
          useAuth.setState({
            consentData: deps.search,
          });
        }
        throw redirect({ to: '/auth/login' });
      });
    return clientResponse;
  },
  component: OpenIdConsentPage,
});
