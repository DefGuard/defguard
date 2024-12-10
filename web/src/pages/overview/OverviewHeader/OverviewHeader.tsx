import { isUndefined } from 'lodash-es';
import { useCallback, useMemo } from 'react';
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
import { useAddStandaloneDeviceModal } from '../modals/AddStandaloneDeviceModal/store';
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
  const openAddStandaloneDeviceModal = useAddStandaloneDeviceModal(
    (s) => s.open,
    shallow,
  );
  const navigate = useNavigate();

  const selectedNetwork = useMemo(
    () => networks?.find((n) => n.id === selectedNetworkId),
    [networks, selectedNetworkId],
  );

  const handleNetworkAction = useCallback(() => {
    if (selectedNetwork) {
      setNetworkPageStore({ selectedNetworkId: selectedNetworkId });
      navigate('../network');
    }
  }, [navigate, selectedNetwork, selectedNetworkId, setNetworkPageStore]);

  // TODO: move to "devices" page
  const renderEditNetworks = useMemo(() => {
    return (
      <>
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.networkOverview.controls.editNetworks()}
          icon={<IconEditNetwork />}
          loading={loading || isUndefined(selectedNetwork)}
          onClick={handleNetworkAction}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={'Add new'}
          icon={
            <svg
              width={18}
              height={18}
              viewBox="0 0 18 18"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                fillRule="evenodd"
                clipRule="evenodd"
                d="M10.5 0L15 0C16.6569 0 18 1.34315 18 3V15C18 16.6569 16.6569 18 15 18H10.5C8.84315 18 7.5 16.6569 7.5 15V3C7.5 1.34315 8.84315 0 10.5 0ZM10.5 1.5C9.67157 1.5 9 2.17157 9 3V15C9 15.8284 9.67157 16.5 10.5 16.5H15C15.8284 16.5 16.5 15.8284 16.5 15V3C16.5 2.17157 15.8284 1.5 15 1.5H10.5Z"
                fill="white"
              />
              <mask
                id="mask0_9115_1065"
                style={{
                  maskType: 'alpha',
                }}
                maskUnits="userSpaceOnUse"
                x={0}
                y={0}
                width={18}
                height={15}
              >
                <path
                  fillRule="evenodd"
                  clipRule="evenodd"
                  d="M4.2196 1.5C3.49863 1.50096 2.80744 1.7878 2.29762 2.29762C1.7878 2.80744 1.50096 3.49863 1.5 4.2196V9.9839C1.50096 10.7047 1.78772 11.3957 2.2974 11.9054C2.80699 12.4149 3.49783 12.7017 4.21847 12.7028C4.21885 12.7028 4.21922 12.7028 4.2196 12.7028H13.7797C13.78 12.7028 13.7804 12.7028 13.7808 12.7028C14.5015 12.7017 15.1925 12.4149 15.7022 11.9053C16.212 11.3956 16.4988 10.7047 16.5 9.98389C16.5 9.98356 16.5 9.98322 16.5 9.98289V4.22035C16.5 4.21997 16.5 4.2196 16.5 4.21922C16.4989 3.49838 16.2121 2.80735 15.7024 2.29762C15.1926 1.7878 14.5014 1.50096 13.7804 1.5H4.2196ZM13.7812 0L4.21875 0C3.10023 0.00119118 2.02787 0.446048 1.23696 1.23696C0.446048 2.02787 0.00119118 3.10023 0 4.21875L0 9.98475C0.00119118 11.1031 0.445969 12.1752 1.23674 12.966C2.02751 13.7568 3.09968 14.2016 4.218 14.2028H13.7812C14.8996 14.2016 15.9719 13.7568 16.7628 12.9661C17.5537 12.1753 17.9986 11.1031 18 9.98475V4.21875C17.9988 3.10023 17.554 2.02787 16.763 1.23696C15.9721 0.446048 14.8998 0.00119118 13.7812 0Z"
                  fill="#899CA8"
                />
              </mask>
              <g mask="url(#mask0_9115_1065)">
                <rect
                  width={7.5}
                  height={17.25}
                  transform="matrix(-1 0 0 1 6.75 -1.5)"
                  fill="white"
                />
              </g>
              <path
                d="M3 17.25C3 16.8358 3.33579 16.5 3.75 16.5H6.75V18H3.75C3.33579 18 3 17.6642 3 17.25V17.25Z"
                fill="white"
              />
              <circle cx={12.75} cy={14.25} r={0.75} fill="white" />
            </svg>
          }
          onClick={() => openAddStandaloneDeviceModal()}
        />
      </>
    );
  }, [
    LL.networkOverview.controls,
    handleNetworkAction,
    loading,
    openAddStandaloneDeviceModal,
    selectedNetwork,
  ]);

  return (
    <>
      {breakpoint !== 'desktop' && (
        <div className="mobile-options">
          <div className="top-row">
            {selectedNetworkId && <GatewaysStatus networkId={selectedNetworkId} />}
            {renderEditNetworks}
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
            {renderEditNetworks}
          </div>
        </header>
      )}
    </>
  );
};
