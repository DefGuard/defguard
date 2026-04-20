import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { cloneDeep } from 'radashi';
import { useCallback } from 'react';
import Skeleton from 'react-loading-skeleton';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { Controls } from '../../shared/components/Controls/Controls';
import { WizardWelcomePage } from '../../shared/components/wizard/WizardWelcomePage/WizardWelcomePage';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { getLocationsDisplayQueryOptions } from '../../shared/query';
import { migrationWizardFinishPromise } from '../../shared/wizard/migrationWizardFinishPromise';
import { useGatewayWizardStore } from '../GatewaySetupPage/useGatewayWizardStore';
import { useMigrationWizardStore } from '../MigrationWizardPage/store/useMigrationWizardStore';
import { MigrationWizardStep } from '../MigrationWizardPage/types';

export const LocationsMigrationWizardPage = () => {
  return (
    <WizardWelcomePage
      containerProps={{
        id: 'locations-migration-page',
      }}
      title={m.migration_wizard_locations_title()}
      displayDocs={false}
      subtitle={m.migration_wizard_locations_subtitle()}
      content={<Content />}
    />
  );
};

const Content = () => {
  const navigate = useNavigate();
  const { data: locationsDisplay, isLoading } = useQuery(getLocationsDisplayQueryOptions);

  const { mutate: updateWizardState } = useMutation({
    mutationFn: api.migration.state.updateMigrationState,
    onSuccess: (_, variables) => {
      useMigrationWizardStore.setState(variables);
    },
    meta: {
      invalidate: ['migration'],
    },
  });

  const { mutate: finish, isPending: finishPending } = useMutation({
    mutationFn: migrationWizardFinishPromise,
    onSuccess: () => {
      Snackbar.default(m.migration_wizard_locations_complete_snackbar());
      navigate({ to: '/vpn-overview', replace: true });
      setTimeout(() => {
        useMigrationWizardStore.getState().resetState();
      }, 2500);
    },
    meta: {
      invalidate: ['session-info'],
    },
  });

  const locationsState = useMigrationWizardStore((s) => s.location_state);

  const currentLocationIndex = useMigrationWizardStore(
    (s) => s.location_state?.locations.indexOf(s.location_state?.current_location) ?? 0,
  );

  const handleStart = useCallback(() => {
    if (!locationsState) return;
    useGatewayWizardStore.getState().start({
      // skip welcome page
      isOnWelcomePage: false,
      isMigrationWizard: true,
      network_id: locationsState.current_location,
    });
    navigate({ to: '/setup-gateway', replace: true });
  }, [locationsState, navigate]);

  const handleSkip = useCallback(() => {
    if (!locationsState) return;
    const currentIndex = locationsState.locations.indexOf(
      locationsState.current_location,
    );
    if (currentIndex === locationsState.locations.length - 1) {
      finish();
      return;
    }
    const state = cloneDeep(locationsState);
    state.current_location = locationsState.locations[currentIndex + 1];
    updateWizardState({
      current_step: MigrationWizardStep.Confirmation,
      location_state: state,
      proxy_url: useMigrationWizardStore.getState().proxy_url,
    });
  }, [locationsState, updateWizardState, finish]);

  if (!locationsState) return null;

  return (
    <>
      <SizedBox height={ThemeSpacing.Lg} />
      <Divider />
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgFaded}>
        {m.migration_wizard_locations_firewall_info()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <Divider />
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgFaded}>
        {m.migration_wizard_locations_progress({
          current: currentLocationIndex + 1,
          total: locationsState.locations.length,
        })}
      </AppText>
      {isLoading && <Skeleton width={160} height={28} />}
      {!isLoading && isPresent(locationsDisplay) && (
        <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgFaded}>
          {locationsDisplay[locationsState.current_location] ??
            m.migration_wizard_locations_unknown()}
        </AppText>
      )}
      <SizedBox height={ThemeSpacing.Xl2} />
      <Divider />
      <SizedBox height={ThemeSpacing.Xl2} />
      <Controls>
        <Button
          text={m.migration_wizard_locations_start_button()}
          disabled={finishPending || isLoading}
          onClick={handleStart}
        />
        <Button
          text={m.migration_wizard_locations_skip_button()}
          variant={'outlined'}
          loading={finishPending}
          disabled={isLoading}
          onClick={handleSkip}
        />
      </Controls>
    </>
  );
};
