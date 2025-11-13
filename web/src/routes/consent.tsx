import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { OpenIdConsentPage } from '../pages/OpenIdConsentPage/OpenIdConsentPage';
import api from '../shared/api/api';

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
  loader: ({ deps }) => {
    return api.openIdClient.getOpenIdClient(deps.search.client_id);
  },
  component: OpenIdConsentPage,
});
