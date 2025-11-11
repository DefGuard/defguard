import { createFileRoute } from '@tanstack/react-router';
import { OpenIdPage } from '../../pages/OpenIdPage/OpenIdPage';
import { getOpenIdClientQueryOptions } from '../../shared/query';

export const Route = createFileRoute('/_authorized/openid')({
  component: OpenIdPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getOpenIdClientQueryOptions);
  },
});
