import { useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Select } from '../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSizeVariant,
} from '../../../../shared/defguard-ui/components/Layout/Select/types';
import { useOverviewTimeSelection } from '../hooks/useOverviewTimeSelection';

const availableFilters: number[] = [1, 2, 4, 6, 8, 12, 16, 24];

export const OverviewTimeSelection = () => {
  const { from, setTimeSelection } = useOverviewTimeSelection();
  const { LL } = useI18nContext();
  const options = useMemo((): SelectOption<number>[] => {
    return availableFilters.map((filter) => ({
      key: filter,
      label: LL.networkOverview.timeRangeSelectionLabel({
        value: filter,
      }),
      value: filter,
    }));
  }, [LL.networkOverview]);

  return (
    <Select
      sizeVariant={SelectSizeVariant.SMALL}
      options={options}
      selected={from}
      onChangeSingle={(value) => {
        setTimeSelection(value);
      }}
    />
  );
};
