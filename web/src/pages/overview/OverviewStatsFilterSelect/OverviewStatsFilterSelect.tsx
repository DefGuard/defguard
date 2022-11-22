import { useMemo } from 'react';

import {
  Select,
  SelectOption,
  SelectStyleVariant,
} from '../../../shared/components/layout/Select/Select';
import { useOverviewStore } from '../hooks/store/useOverviewStore';

export const OverviewStatsFilterSelect = () => {
  const filterValue = useOverviewStore((state) => state.statsFilter);
  const setOverviewStore = useOverviewStore((state) => state.setState);

  const getCurrentValue = useMemo(
    () => selectOptions.find((o) => o.value === filterValue),
    [filterValue]
  );

  return (
    <Select
      options={selectOptions}
      selected={getCurrentValue}
      multi={false}
      styleVariant={SelectStyleVariant.LIGHT}
      onChange={(o) => {
        if (o && !Array.isArray(o)) {
          setOverviewStore({ statsFilter: o.value });
        }
      }}
    />
  );
};

const selectOptions: SelectOption<number>[] = [
  {
    value: 1,
    label: '1H',
    key: 1,
  },
  {
    value: 2,
    label: '2H',
    key: 2,
  },
  {
    value: 4,
    label: '4H',
    key: 4,
  },
  {
    value: 6,
    label: '6H',
    key: 6,
  },
  {
    value: 8,
    label: '8H',
    key: 8,
  },
  {
    value: 10,
    label: '10H',
    key: 10,
  },
  {
    value: 12,
    label: '12H',
    key: 12,
  },
  {
    value: 24,
    label: '24H',
    key: 24,
  },
];
