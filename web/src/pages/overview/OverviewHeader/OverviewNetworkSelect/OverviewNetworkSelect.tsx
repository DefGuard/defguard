import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Select } from '../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../shared/defguard-ui/components/Layout/Select/types';
import { useOverviewStore } from '../../hooks/store/useOverviewStore';

export const OverViewNetworkSelect = () => {
  const { LL } = useI18nContext();
  const [selectedNetworkId, networks] = useOverviewStore(
    (state) => [state.selectedNetworkId, state.networks],
    shallow,
  );
  const setOverviewStore = useOverviewStore((state) => state.setState);

  const options = useMemo((): SelectOption<number>[] => {
    if (networks) {
      return networks.map((n) => ({
        label: n.name,
        key: n.id,
        value: n.id,
      }));
    }
    return [];
  }, [networks]);

  const renderSelected = useCallback(
    (selected: number): SelectSelectedValue => {
      const option = options.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exist");
      return {
        key: selected,
        displayValue: option.label,
      };
    },
    [options],
  );

  useEffect(() => console.log(selectedNetworkId, options), [selectedNetworkId, options]);

  return (
    <Select
      renderSelected={renderSelected}
      placeholder={LL.networkOverview.controls.selectNetwork.placeholder()}
      loading={isUndefined(networks) || networks.length === 0}
      selected={selectedNetworkId}
      options={options}
      onChangeSingle={(res) => {
        setOverviewStore({ selectedNetworkId: res });
      }}
    />
  );
};
