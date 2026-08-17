import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { Suspense } from 'react';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { TableSkeleton } from '../../shared/components/skeleton/TableSkeleton/TableSkeleton';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import { getMfaFlowsQueryOptions } from '../../shared/query';
import { MfaFlowsTable } from './MfaFlowsTable';

/** Loads and renders either the configured MFA flows or the first-use empty state. */
const MfaPageContent = () => {
  const navigate = useNavigate();
  const { data: flows } = useSuspenseQuery(getMfaFlowsQueryOptions);
  const addMfaFlowButtonProps: ButtonProps = {
    text: m.mfa_flows_button_add(),
    iconLeft: 'plus',
    testId: 'add-mfa-flow',
    onClick: () => {
      void navigate({ to: '/mfa/add-flow' });
    },
  };

  return (
    <TablePageLayout>
      {flows.length > 0 ? (
        <MfaFlowsTable flows={flows} addButtonProps={addMfaFlowButtonProps} />
      ) : (
        <EmptyStateFlexible
          icon="posture-checks"
          title={m.mfa_flows_empty_title()}
          subtitle={m.mfa_flows_empty_subtitle()}
          primaryAction={addMfaFlowButtonProps}
        />
      )}
    </TablePageLayout>
  );
};

export const MfaPage = () => {
  return (
    <Page id="mfa-page" title={m.cmp_nav_item_mfa()}>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Suspense fallback={<TableSkeleton />}>
        <MfaPageContent />
      </Suspense>
    </Page>
  );
};
