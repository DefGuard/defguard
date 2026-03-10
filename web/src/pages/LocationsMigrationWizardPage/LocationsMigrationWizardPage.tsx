import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { cloneDeep } from 'radashi';
import { useCallback } from 'react';
import Skeleton from 'react-loading-skeleton';
import api from '../../shared/api/api';
import { Controls } from '../../shared/components/Controls/Controls';
import { WizardWelcomePage } from '../../shared/components/wizard/WizardWelcomePage/WizardWelcomePage';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { getLocationsDisplayQueryOptions } from '../../shared/query';
import { migrationWizardFinishPromise } from '../../shared/wizard/migrationWizardFinishPromise';
import { useGatewayWizardStore } from '../GatewaySetupPage/useGatewayWizardStore';
import { useMigrationWizardStore } from '../MigrationWizardPage/store/useMigrationWizardStore';
import { MigrationWizardStep } from '../MigrationWizardPage/types';

const subtitle = `We will verify the connection, ensure the port is open, and confirm the gateway is running the correct version. Any errors will be displayed, allowing you to fix issues and retry during the process.`;

export const LocationsMigrationWizardPage = () => {
  return (
    <WizardWelcomePage
      containerProps={{
        id: 'locations-migration-page',
      }}
      title={`VPN Locations Migration`}
      displayDocs={false}
      subtitle={subtitle}
      content={<Content />}
    />
  );
};

const Content = () => {
  const navigate = useNavigate();
  const { data: locationsDisplay, isLoading } = useQuery(getLocationsDisplayQueryOptions);

  const { mutate: updateWizardState } = useMutation({
    mutationFn: api.migration.state.updateMigrationState,
    meta: {
      invalidate: ['migration'],
    },
  });

  const { mutate: finish, isPending: finishPending } = useMutation({
    mutationFn: migrationWizardFinishPromise,
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
    state.current_location + 1;
    updateWizardState({
      current_step: MigrationWizardStep.Confirmation,
      location_state: state,
    });
  }, [locationsState, updateWizardState, finish]);

  if (!locationsState) return null;

  return (
    <>
      <Divider spacing={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgFaded}>
        {`By clicking the button bellow, you confirm that the required firewall changes have been made and that the Core can connect to this gateway on TCP port 5055. In case you have any question please read our documentation following the link in the bottom section.`}
      </AppText>
      <Divider spacing={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgFaded}>
        {`Migrate ${currentLocationIndex + 1} of ${locationsState.locations.length} location(s):`}
      </AppText>
      {isLoading && <Skeleton width={160} height={28} />}
      {!isLoading && isPresent(locationsDisplay) && (
        <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgFaded}>
          {locationsDisplay[locationsState.current_location] ?? `Unknown`}
        </AppText>
      )}
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        <Button
          text={`Start migration`}
          disabled={finishPending || isLoading}
          onClick={handleStart}
        />
        <Button
          text={`Skip location`}
          variant={'outlined'}
          loading={finishPending}
          disabled={isLoading}
          onClick={handleSkip}
        />
      </Controls>
    </>
  );
};
