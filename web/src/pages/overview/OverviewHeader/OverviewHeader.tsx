import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../i18n/i18n-react';
import { NetworkGatewaysStatus } from '../../../shared/components/network/GatewaysStatus/NetworkGatewaysStatus/NetworkGatewaysStatus';
import { deviceBreakpoints } from '../../../shared/constants';
import { EditLocationsSettingsButton } from '../../overview-index/components/EditLocationsSettingsButton/EditLocationsSettingsButton';
import { OverviewNetworkSelection } from '../../overview-index/components/OverviewNetworkSelection/OverviewNetworkSelection';
import { OverviewTimeSelection } from '../../overview-index/components/OverviewTimeSelection/OverviewTimeSelection';
import { useOverviewStore } from '../hooks/store/useOverviewStore';

export const OverviewHeader = () => {
  const { LL } = useI18nContext();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [selectedNetworkId] = useOverviewStore(
    (state) => [state.selectedNetworkId, state.networks],
    shallow,
  );

  return (
    <>
      {breakpoint !== 'desktop' && (
        <div className="mobile-options">
          <div className="top-row">
            {selectedNetworkId && <NetworkGatewaysStatus networkId={selectedNetworkId} />}
            <EditLocationsSettingsButton />
          </div>
          <OverviewNetworkSelection />
          <OverviewTimeSelection />
        </div>
      )}
      {breakpoint === 'desktop' && (
        <header>
          <h1>{LL.networkOverview.pageTitle()}</h1>
          <div className="controls">
            <OverviewNetworkSelection />
            <OverviewTimeSelection />
            <EditLocationsSettingsButton />
          </div>
        </header>
      )}
    </>
  );
};
