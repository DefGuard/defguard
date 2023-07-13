import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Select, SelectOption } from '../../../../shared/components/layout/Select/Select';
import { useOverviewStore } from '../../hooks/store/useOverviewStore';

export const OverViewNetworkSelect = () => {
  const { LL } = useI18nContext();
  const [selectedNetworkId, networks] = useOverviewStore(
    (state) => [state.selectedNetworkId, state.networks],
    shallow
  );
  const setOverviewStore = useOverviewStore((state) => state.setState);

  const selected = useMemo((): SelectOption<number> | undefined => {
    const network = networks?.find((n) => n.id === selectedNetworkId);
    if (network) {
      return {
        label: network.name,
        value: network.id,
        key: network.id,
      };
    }
    return undefined;
  }, [networks, selectedNetworkId]);

  const options = useMemo((): SelectOption<number>[] | undefined => {
    if (networks) {
      return networks.map((n) => ({
        label: n.name,
        key: n.id,
        value: n.id,
      }));
    }
    return undefined;
  }, [networks]);

  return (
    <Select
      placeholder={LL.networkOverview.controls.selectNetwork.placeholder()}
      loading={isUndefined(networks) || networks.length === 0}
      selected={selected}
      options={options}
      onChange={(option) => {
        if (!Array.isArray(option) && networks) {
          setOverviewStore({ selectedNetworkId: option?.value });
        }
      }}
    />
  );
};
