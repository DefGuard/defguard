import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { cloneDeep } from 'radashi';
import { useCallback, useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { MigrationWizardLocationState } from '../../../shared/api/types';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import {
  getLocationsDisplayQueryOptions,
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

  const isMigrationWizard = useGatewayWizardStore((s) => s.isMigrationWizard);
  const migrationLocationState = useMigrationWizardStore((s) => s.location_state);

  const { data: locationsDisplay } = useQuery(getLocationsDisplayQueryOptions);

  const isLastLocationToMigrate = useMemo(() => {
    if (!isMigrationWizard || !migrationLocationState) return false;
    const currentLocationIndex = migrationLocationState.locations.indexOf(
      migrationLocationState.current_location,
    );
    return currentLocationIndex === migrationLocationState.locations.length - 1;
  }, [isMigrationWizard, migrationLocationState]);

  const currentLocationName = useMemo(() => {
    if (!isMigrationWizard || !migrationLocationState) return '';
    return (
      locationsDisplay?.[migrationLocationState.current_location] ??
      `#${migrationLocationState.current_location}`
    );
  }, [isMigrationWizard, locationsDisplay, migrationLocationState]);

  const cardHeader = useMemo((): { title: string; subtitle: string } => {
    if (!isMigrationWizard) {
      return {
        title: m.gateway_setup_confirmation_title(),
        subtitle: m.gateway_setup_confirmation_subtitle(),
      };
    }

    if (isLastLocationToMigrate) {
      return {
        title: m.gateway_setup_confirmation_migration_last_title(),
        subtitle: m.gateway_setup_confirmation_migration_last_subtitle(),
      };
    }

    return {
      title: m.gateway_setup_confirmation_migration_title(),
      subtitle: m.gateway_setup_confirmation_migration_subtitle({
        location: currentLocationName,
      }),
    };
  }, [currentLocationName, isLastLocationToMigrate, isMigrationWizard]);

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
      const currentLocationIndex = locationState.locations.indexOf(
        locationState.current_location,
      );
      // finish migration
      if (currentLocationIndex === locationState.locations.length - 1) {
        await migrationWizardFinishPromise();
        await queryClient.invalidateQueries({
          queryKey: getSessionInfoQueryOptions.queryKey,
        });
        Snackbar.default(m.migration_wizard_finish_success_snackbar());
        await navigate({ to: '/vpn-overview', replace: true });
        setTimeout(() => {
          useMigrationWizardStore.getState().resetState();
        }, 2500);
        return;
      }
      // otherwise open next location migration
      locationState.current_location = locationState.locations[currentLocationIndex + 1];
      await api.migration.state.updateMigrationState({
        current_step: MigrationWizardStep.Confirmation,
        location_state: locationState,
        proxy_url: useMigrationWizardStore.getState().proxy_url,
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

  const { mutate: finish, isPending: finishPending } = useMutation({
    mutationFn: handleFinish,
    onError: (e) => {
      Snackbar.error(m.gateway_setup_adoption_error_default());
      console.error(e);
    },
  });

  return (
    <WizardCard>
      <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgSuccess}>
        {cardHeader.title}
      </AppText>
      <SizedBox height={ThemeSpacing.Sm} />
      <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgNeutral}>
        {cardHeader.subtitle}
      </AppText>
      <Divider spacing={ThemeSpacing.Xl2} />
      {!isMigrationWizard && (
        <>
          <ActionCard
            title={m.gateway_setup_add_multiple_gateways_title()}
            subtitle={m.gateway_setup_add_multiple_gateways_subtitle()}
            imageSrc={addMoreImage}
          />
          <Controls>
            <div className="right">
              <Button
                text={m.gateway_setup_controls_add_another_gateway()}
                onClick={handleBack}
                disabled={finishPending}
                variant="outlined"
              />
              <Button
                text={m.gateway_setup_controls_go_to_locations()}
                onClick={() => finish()}
                loading={finishPending}
              />
            </div>
          </Controls>
        </>
      )}
      {isMigrationWizard && (
        <Controls>
          <div className="right">
            <Button
              text={
                isLastLocationToMigrate
                  ? m.migration_wizard_confirmation_finish_control()
                  : m.gateway_setup_adoption_controls_continue()
              }
              onClick={() => finish()}
              loading={finishPending}
            />
          </div>
        </Controls>
      )}
    </WizardCard>
  );
};
