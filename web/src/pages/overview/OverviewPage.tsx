import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isNull, isUndefined } from 'lodash-es';
import { useEffect } from 'react';
import { useNavigate } from 'react-router';
import useBreakpoint from 'use-breakpoint';
import shallow from 'zustand/shallow';

import { Action } from '../../shared/components/layout/Action/Action';
import Button, {
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { IconEditNetwork } from '../../shared/components/svg';
import { deviceBreakpoints } from '../../shared/constants';
import { apiToWizardNetwork } from '../../shared/helpers/apiToWizardNetwork';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useModalStore } from '../../shared/hooks/store/useModalStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { OverviewLayoutType } from '../../shared/types';
import GatewaySetupModal from '../vpn/modals/GatewaySetupModal/GatewaySetupModal';
import { useWizardStore } from '../vpn/Wizard/WizardSteps/store';
import { getNetworkStatsFilterValue } from './helpers/stats';
import { useOverviewStore } from './hooks/store/useOverviewStore';
import { OverviewActivityStream } from './OverviewActivityStream/OverviewActivityStream';
import { OverviewConnectedUsers } from './OverviewConnectedUsers/OverviewConnectedUsers';
import { OverviewStats } from './OverviewStats/OverviewStats';
import { OverviewStatsFilterSelect } from './OverviewStatsFilterSelect/OverviewStatsFilterSelect';
import { OverviewViewSelect } from './OverviewViewSelect/OverviewViewSelect';

export const OverviewPage = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const viewMode = useOverviewStore((state) => state.viewMode);
  const setOverViewStore = useOverviewStore((state) => state.setState);
  const statsFilter = useOverviewStore((state) => state.statsFilter);
  const wizardCompleted = useAppStore((state) => state.wizardCompleted);

  const setGatewaySetupModal = useModalStore(
    (state) => state.setGatewaySetupModal
  );
  const {
    network: { getNetworks, getUsersStats, getNetworkStats },
  } = useApi();

  const [resetWizardStore, setNetwork, setWizardState] = useWizardStore(
    (state) => [state.resetStore, state.setNetwork, state.setState],
    shallow
  );

  const navigate = useNavigate();

  const settings = useAppStore((state) => state.settings);
  if (!settings?.wireguard_enabled) navigate('/');

  const { data: networkStats } = useQuery(
    [QueryKeys.FETCH_NETWORK_STATS, statsFilter],
    () => getNetworkStats({ from: getNetworkStatsFilterValue(statsFilter) })
  );

  const { data: networkUsersStats } = useQuery(
    [QueryKeys.FETCH_NETWORK_USERS_STATS, statsFilter],
    () => getUsersStats({ from: getNetworkStatsFilterValue(statsFilter) }),
    {
      enabled: !isUndefined(statsFilter),
    }
  );

  const {
    data: networks,
    isLoading: networksLoading,
    isSuccess: networksSuccess,
  } = useQuery([QueryKeys.FETCH_NETWORKS], getNetworks, {
    onSuccess: (networks) => {
      const network = networks[0];
      if (network) {
        if (isNull(network.connected_at)) {
          setGatewaySetupModal({ visible: true });
        }
      }
    },
  });

  useEffect(() => {
    if (breakpoint === 'mobile' && viewMode === OverviewLayoutType.LIST) {
      setOverViewStore({ viewMode: OverviewLayoutType.GRID });
    }
    if (breakpoint === 'tablet' && viewMode === OverviewLayoutType.GRID) {
      setOverViewStore({ viewMode: OverviewLayoutType.LIST });
    }
  }, [viewMode, breakpoint, setOverViewStore]);

  useEffect(() => {
    if (!networksLoading && networksSuccess) {
      if (networks?.length == 0 && !wizardCompleted) {
        resetWizardStore({
          editMode: false,
        });
        navigate('/admin/wizard', { replace: true });
      }
    }
  }, [
    networks,
    navigate,
    networksLoading,
    resetWizardStore,
    networksSuccess,
    wizardCompleted,
  ]);

  return (
    <>
      <PageContainer id="network-overview-page">
        {breakpoint !== 'desktop' && (
          <div className="mobile-options">
            <OverviewViewSelect />
            <OverviewStatsFilterSelect />
          </div>
        )}
        {breakpoint === 'desktop' && (
          <header>
            <h1>Network overview</h1>
            <OverviewViewSelect />
            <OverviewStatsFilterSelect />
            <Action
              onClick={() => setGatewaySetupModal({ visible: true })}
              className="docker-command"
            >
              Docker run command
            </Action>
            <Button
              styleVariant={ButtonStyleVariant.STANDARD}
              text={'Edit network settings'}
              icon={<IconEditNetwork />}
              disabled={isUndefined(networks) || networksLoading}
              onClick={() => {
                if (networks) {
                  setNetwork(apiToWizardNetwork(networks[0]));
                  setWizardState({
                    editMode: true,
                  });
                  navigate('/admin/wizard/1', {
                    replace: true,
                  });
                }
              }}
            />
          </header>
        )}
        {networkStats && networkUsersStats && (
          <OverviewStats
            usersStats={networkUsersStats}
            networkStats={networkStats}
          />
        )}
        <div className="bottom-row">
          <OverviewConnectedUsers stats={networkUsersStats} />
          <OverviewActivityStream />
        </div>
      </PageContainer>
      <GatewaySetupModal />
    </>
  );
};
