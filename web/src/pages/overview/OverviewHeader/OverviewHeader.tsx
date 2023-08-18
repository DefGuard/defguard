import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import { GatewaysStatus } from '../../../shared/components/network/GatewaysStatus/GatewaysStatus';
import IconEditNetwork from '../../../shared/components/svg/IconEditNetwork';
import { deviceBreakpoints } from '../../../shared/constants';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../shared/defguard-ui/components/Layout/Button/types';
import { useNetworkPageStore } from '../../network/hooks/useNetworkPageStore';
import { useOverviewStore } from '../hooks/store/useOverviewStore';
import { OverviewStatsFilterSelect } from '../OverviewStatsFilterSelect/OverviewStatsFilterSelect';
import { OverViewNetworkSelect } from './OverviewNetworkSelect/OverviewNetworkSelect';

type Props = {
  loading?: boolean;
};

export const OverviewHeader = ({ loading = false }: Props) => {
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const setNetworkPageStore = useNetworkPageStore((state) => state.setState);
  const [selectedNetworkId, networks] = useOverviewStore(
    (state) => [state.selectedNetworkId, state.networks],
    shallow,
  );
  const navigate = useNavigate();

  const selectedNetwork = useMemo(
    () => networks?.find((n) => n.id === selectedNetworkId),
    [networks, selectedNetworkId],
  );

  const handleNetworkAction = () => {
    if (selectedNetwork) {
      setNetworkPageStore({ selectedNetworkId: selectedNetworkId });
      navigate('../network');
    }
  };

  const renderEditNetworks = () => {
    return (
      <Button
        styleVariant={ButtonStyleVariant.STANDARD}
        text={LL.networkOverview.controls.editNetworks()}
        icon={<IconEditNetwork />}
        loading={loading || isUndefined(selectedNetwork)}
        onClick={handleNetworkAction}
      />
    );
  };

  return (
    <>
      {breakpoint !== 'desktop' && (
        <div className="mobile-options">
          <div className="top-row">
            {selectedNetworkId && <GatewaysStatus networkId={selectedNetworkId} />}
            {renderEditNetworks()}
          </div>
          <OverViewNetworkSelect />
          <OverviewStatsFilterSelect />
        </div>
      )}
      {breakpoint === 'desktop' && (
        <header>
          <h1>{LL.networkOverview.pageTitle()}</h1>
          <div className="controls">
            <OverViewNetworkSelect />
            <OverviewStatsFilterSelect />
            {renderEditNetworks()}
          </div>
        </header>
      )}
    </>
  );
};
