import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getWebhooksQueryOptions } from '../../shared/query';
import { CeWebhookModal } from './modals/CEWebhookModal/CEWebhookModal';
import { WebhooksTable } from './WebhooksTable';

export const WebhooksPage = () => {
  const { data: webhooks } = useSuspenseQuery(getWebhooksQueryOptions);

  return (
    <>
      <Page id="webhooks-page" title="Webhooks">
        <SizedBox height={ThemeSpacing.Xl3} />
        <TablePageLayout>
          <WebhooksTable webhooks={webhooks} />
        </TablePageLayout>
      </Page>
      <CeWebhookModal />
    </>
  );
};
