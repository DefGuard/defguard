import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { LocationsTable } from './components/LocationsTable';
import './style.scss';
import { useEffect } from 'react';
import api from '../../shared/api/api';
import { GatewaySetupModal } from '../../shared/components/modals/GatewaySetupModal/GatewaySetupModal';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { getLocationsQueryOptions } from '../../shared/query';
import { useLocationsPageStore } from './hooks/useLocationsPage';
import { AddLocationModal } from './modals/AddLocationModal/AddLocationModal';

export const LocationsPage = () => {
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);

  // Auto start gateway setup modal
  useEffect(() => {
    const gatewaySetupStartup = useLocationsPageStore.getState().networkGatewayStartup;

    if (isPresent(gatewaySetupStartup)) {
      const handleOpen = async () => {
        const enrollData = (await api.location.getGatewayToken(gatewaySetupStartup)).data;
        openModal(ModalName.GatewaySetup, {
          data: enrollData,
          networkId: gatewaySetupStartup,
        });
        useLocationsPageStore.setState({
          networkGatewayStartup: undefined,
        });
      };
      handleOpen();
    }
  }, []);

  return (
    <>
      <Page title="Locations" id="locations-page">
        <LocationsTable locations={locations} />
      </Page>
      <GatewaySetupModal />
      <AddLocationModal />
    </>
  );
};
