import {
  queryOptions,
  useMutation,
  useQueryClient,
  useSuspenseQuery,
} from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type { MigrationWizardApiState } from '../../../../shared/api/types';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { externalLink } from '../../../../shared/constants';
import { ActionableSection } from '../../../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { RenderMarkdown } from '../../../../shared/defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import {
  getMigrationStateQueryOptions,
  getSessionInfoQueryOptions,
} from '../../../../shared/query';
import { migrationWizardFinishPromise } from '../../../../shared/wizard/migrationWizardFinishPromise';
import { useMigrationWizardStore } from '../../store/useMigrationWizardStore';
import { MigrationWizardStep } from '../../types';
import prepareNetworkImage from './assets/prepare-network.png';

const qOptions = queryOptions({
  queryKey: ['network', 'display', 'list'],
  queryFn: api.location.getLocationsDisplay,
  select: (resp) => resp.data,
});

export const MigrationWizardConfirmationStep = () => {
  const queryClient = useQueryClient();
  const [confirm, setConfirm] = useState(false);
  const navigate = useNavigate();

  const { data: locationsDisplay } = useSuspenseQuery(qOptions);

  const locationsMigrationNeeded = locationsDisplay.length > 0;

  const { mutate: finish, isPending: finishPending } = useMutation({
    mutationFn: migrationWizardFinishPromise,
    onSuccess: async () => {
      Snackbar.default(m.migration_wizard_finish_success_snackbar());
      await navigate({ to: '/vpn-overview', replace: true });
      setTimeout(() => {
        useMigrationWizardStore.getState().resetState();
      }, 2500);
    },
    onError: (e) => {
      Snackbar.error(m.migration_wizard_finish_error_snackbar());
      console.error(e);
    },
    meta: {
      invalidate: [['settings'], ['session-info'], ['me']],
    },
  });

  const { mutate: startLocationsMigration, isPending: startLocationsPending } =
    useMutation({
      mutationFn: async () => {
        const locationsIds = locationsDisplay.map((key) => key.id);
        const migrationLocationState: MigrationWizardApiState['location_state'] = {
          current_location: locationsIds[0],
          locations: locationsIds,
        };
        await api.migration.state.updateMigrationState({
          current_step: MigrationWizardStep.Confirmation,
          location_state: migrationLocationState,
        });
        await queryClient.invalidateQueries({
          queryKey: getMigrationStateQueryOptions.queryKey,
        });
        await queryClient.invalidateQueries({
          queryKey: getSessionInfoQueryOptions.queryKey,
        });
        useMigrationWizardStore.setState({
          location_state: migrationLocationState,
        });
        await navigate({ to: '/migration/locations', replace: true });
      },
    });

  return (
    <WizardCard>
      <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgSuccess}>
        {m.migration_wizard_confirmation_title()}
      </AppText>
      <SizedBox height={ThemeSpacing.Sm} />
      <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgNeutral}>
        {m.migration_wizard_confirmation_subtitle()}
      </AppText>
      <Divider spacing={ThemeSpacing.Xl2} />
      <AppText font={TextStyle.TBodyPrimary500} color={ThemeVariable.FgFaded}>
        {m.migration_wizard_confirmation_locations_info({
          count: locationsDisplay.length ?? '',
        })}
      </AppText>
      <SizedBox height={ThemeSpacing.Md} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
        {m.migration_wizard_confirmation_architecture_change_info()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <ul id="upgrade-guide-list">
        <li>{m.migration_wizard_confirmation_rule_1()}</li>
        <li>{m.migration_wizard_confirmation_rule_2()}</li>
      </ul>
      <Divider spacing={ThemeSpacing.Lg} />
      <RenderMarkdown
        containerProps={{
          id: 'confirm-improve-notice',
        }}
        content={m.migration_wizard_confirmation_security_notice_markdown({
          link: externalLink.defguard.docs,
        })}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <ActionableSection
        imageSrc={prepareNetworkImage}
        title={m.migration_wizard_confirmation_prepare_network_title()}
        subtitle={m.migration_wizard_confirmation_prepare_network_subtitle()}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <Checkbox
        active={confirm}
        onClick={() => {
          setConfirm((s) => !s);
        }}
        text={m.migration_wizard_confirmation_checkbox_label()}
      />
      <Controls>
        <div className="right">
          {!locationsMigrationNeeded && (
            <Button
              variant="primary"
              text={m.controls_finish()}
              disabled={!confirm}
              loading={finishPending}
              onClick={() => {
                finish();
              }}
            />
          )}
          {locationsMigrationNeeded && (
            <Button
              variant="primary"
              text={m.controls_continue()}
              disabled={!confirm}
              loading={startLocationsPending}
              onClick={() => {
                startLocationsMigration();
              }}
            />
          )}
        </div>
      </Controls>
    </WizardCard>
  );
};
