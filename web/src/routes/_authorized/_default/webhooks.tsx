import { createFileRoute } from '@tanstack/react-router';
import { WebhooksPage } from '../../../pages/WebhooksPage/WebhooksPage';
import { getWebhooksQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/webhooks')({
  component: WebhooksPage,
  loader: ({ context }) => {
    return context.queryClient.ensureQueryData(getWebhooksQueryOptions);
  },
});
