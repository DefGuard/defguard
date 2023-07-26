import { useMemo } from 'react';
import { useNavigate } from 'react-router';

import { useI18nContext } from '../../../i18n/i18n-react';
import { CardTabs } from '../../../shared/components/layout/CardTabs/CardTabs';
import { useWizardStore } from '../../wizard/hooks/useWizardStore';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkTabs = () => {
  const navigate = useNavigate();
  const { LL } = useI18nContext();
  const networks = useNetworkPageStore((state) => state.networks);
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);
  const setPageState = useNetworkPageStore((state) => state.setState);
  const resetWizardState = useWizardStore((state) => state.resetState);
  const tabs = useMemo(
    () =>
      networks.map((n) => ({
        key: n.id,
        onClick: () => {
          if (n.id !== selectedNetworkId) {
            setPageState({ selectedNetworkId: n.id });
          }
        },
        content: n.name,
        active: n.id === selectedNetworkId,
      })),
    [networks, selectedNetworkId, setPageState]
  );

  return (
    <CardTabs
      tabs={tabs}
      createContent={LL.networkPage.addNetwork()}
      onCreate={() => {
        resetWizardState();
        navigate('/admin/wizard', { replace: true });
      }}
      loading={!networks || networks.length === 0}
    />
  );
};
