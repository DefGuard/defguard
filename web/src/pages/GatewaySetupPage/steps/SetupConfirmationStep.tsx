import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { cloneDeep } from 'radashi';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { MigrationWizardLocationState } from '../../../shared/api/types';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import {
  getMigrationStateQueryOptions,
  getSessionInfoQueryOptions,
} from '../../../shared/query';
import { migrationWizardFinishPromise } from '../../../shared/wizard/migrationWizardFinishPromise';
import { useMigrationWizardStore } from '../../MigrationWizardPage/store/useMigrationWizardStore';
import { MigrationWizardStep } from '../../MigrationWizardPage/types';
import addMoreImage from '../assets/add_more.svg';
import { useGatewayWizardStore } from '../useGatewayWizardStore';

export const SetupConfirmationStep = () => {
  const queryClient = useQueryClient();
  const navigate = useNavigate();

  const handleBack = () => {
    const networkId = useGatewayWizardStore.getState().network_id;
    useGatewayWizardStore.getState().reset();
    useGatewayWizardStore.getState().start({ network_id: networkId });
  };

  const handleFinish = useCallback(async () => {
    if (useGatewayWizardStore.getState().isMigrationWizard) {
      const locationState = cloneDeep(
        useMigrationWizardStore.getState().location_state as MigrationWizardLocationState,
      );
      // finish migration
      if (locationState.current_location === locationState.locations.length - 1) {
        await migrationWizardFinishPromise();
        await queryClient.invalidateQueries({
          queryKey: getSessionInfoQueryOptions.queryKey,
        });
        Snackbar.success(`Migration completed`);
        await navigate({ to: '/vpn-overview', replace: true });
        setTimeout(() => {
          useMigrationWizardStore.getState().resetState();
        }, 2500);
        return;
      }
      // otherwise open next location migration
      locationState.current_location + 1;
      await api.migration.state.updateMigrationState({
        current_step: MigrationWizardStep.Confirmation,
        location_state: locationState,
      });
      await queryClient.invalidateQueries({
        queryKey: getMigrationStateQueryOptions.queryKey,
      });
      useMigrationWizardStore.setState({
        location_state: locationState,
      });
      await navigate({ to: '/migration/locations', replace: true });
      return;
    } else {
      await navigate({ to: '/locations', replace: true });
    }
    setTimeout(() => {
      useGatewayWizardStore.getState().reset();
    }, 100);
  }, [navigate, queryClient]);

  return (
    <WizardCard>
      <h2>{m.gateway_setup_confirmation_title()}</h2>
      <SizedBox height={ThemeSpacing.Sm} />
      <p>{m.gateway_setup_confirmation_subtitle()}</p>
      <Divider spacing={ThemeSpacing.Xl2} />
      <ActionCard
        title={m.gateway_setup_add_multiple_gateways_title()}
        subtitle={m.gateway_setup_add_multiple_gateways_subtitle()}
        imageSrc={addMoreImage}
      />
      <ModalControls
        cancelProps={{
          text: m.gateway_setup_controls_add_another_gateway(),
          onClick: handleBack,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.gateway_setup_controls_go_to_locations(),
          onClick: handleFinish,
        }}
      />
    </WizardCard>
  );
};
