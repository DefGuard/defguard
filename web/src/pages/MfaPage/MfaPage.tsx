import { useNavigate } from '@tanstack/react-router';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';

export const MfaPage = () => {
  const navigate = useNavigate();
  const addMfaFlowButtonProps: ButtonProps = {
    text: m.mfa_flows_button_add(),
    iconLeft: 'plus',
    testId: 'add-mfa-flow',
    onClick: () => {
      void navigate({ to: '/mfa/add-flow' });
    },
  };

  return (
    <Page id="mfa-page" title={m.cmp_nav_item_mfa()}>
      <SizedBox height={ThemeSpacing.Xl3} />
      <TablePageLayout>
        <EmptyStateFlexible
          icon="posture-checks"
          title={m.mfa_flows_empty_title()}
          subtitle={m.mfa_flows_empty_subtitle()}
          primaryAction={addMfaFlowButtonProps}
        />
      </TablePageLayout>
    </Page>
  );
};
