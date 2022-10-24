import { useMemo } from 'react';
import Select from 'react-select';

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
      className="custom-select"
      classNamePrefix="rs"
      options={selectOptions}
      value={getCurrentValue}
      onChange={(o) => {
        if (o?.value) {
          setOverviewStore({
            statsFilter: o.value,
          });
        }
      }}
    />
  );
};

const selectOptions = [
  {
    value: 1,
    label: '1H',
  },
  {
    value: 2,
    label: '2H',
  },
  {
    value: 4,
    label: '4H',
  },
  {
    value: 6,
    label: '6H',
  },
  {
    value: 8,
    label: '8H',
  },
  {
    value: 10,
    label: '10H',
  },
  {
    value: 12,
    label: '12H',
  },
  {
    value: 24,
    label: '24H',
  },
];
