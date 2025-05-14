import { useNavigate, useParams } from 'react-router';

import { useI18nContext } from '../../../../i18n/i18n-react';
import IconEditNetwork from '../../../../shared/components/svg/IconEditNetwork';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { useNetworkPageStore } from '../../../network/hooks/useNetworkPageStore';

export const EditLocationsSettingsButton = () => {
  const { LL } = useI18nContext();
  const { networkId } = useParams();
  const navigate = useNavigate();
  const selectedNetwork = parseInt(networkId ?? '');
  const setNetworkPageStore = useNetworkPageStore((s) => s.setState);

  const handleClick = () => {
    if (!isNaN(selectedNetwork)) {
      setNetworkPageStore({
        selectedNetworkId: selectedNetwork,
      });
    }
    setNetworkPageStore({
      selectedNetworkId: undefined,
    });
    navigate('/admin/network');
  };

  return (
    <Button
      id="overview-edit-locations-settings-btn"
      styleVariant={ButtonStyleVariant.STANDARD}
      text={LL.networkOverview.controls.editNetworks()}
      icon={<IconEditNetwork />}
      onClick={handleClick}
      size={ButtonSize.SMALL}
    />
  );
};
