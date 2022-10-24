import { useMemo } from 'react';
import Select from 'react-select';
import useBreakpoint from 'use-breakpoint';

import { deviceBreakpoints } from '../../../shared/constants';
import { OverviewLayoutType } from '../../../shared/types';
import { useOverviewStore } from '../hooks/store/useOverviewStore';

export const OverviewViewSelect = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const viewMode = useOverviewStore((state) => state.viewMode);
  const setOverViewStore = useOverviewStore((state) => state.setState);
  const getSelectOptions = useMemo(() => {
    if (breakpoint === 'mobile') {
      return [
        {
          value: OverviewLayoutType.GRID,
          label: 'Grid view',
        },
        { value: OverviewLayoutType.LIST, label: 'List view', disabled: true },
        { value: OverviewLayoutType.MAP, label: 'Map view', disabled: true },
      ];
    }
    if (breakpoint === 'tablet') {
      return [
        {
          value: OverviewLayoutType.GRID,
          label: 'Grid view',
          disabled: true,
        },
        { value: OverviewLayoutType.LIST, label: 'List view', disabled: false },
        { value: OverviewLayoutType.MAP, label: 'Map view', disabled: true },
      ];
    }
    return [
      {
        value: OverviewLayoutType.GRID,
        label: 'Grid view',
      },
      { value: OverviewLayoutType.LIST, label: 'List view' },
      { value: OverviewLayoutType.MAP, label: 'Map view', disabled: true },
    ];
  }, [breakpoint]);

  const getSelectValue = useMemo(() => {
    return getSelectOptions.find((o) => o.value === viewMode);
  }, [getSelectOptions, viewMode]);

  return (
    <Select
      options={getSelectOptions}
      className="custom-select"
      classNamePrefix="rs"
      value={getSelectValue}
      onChange={(option) => {
        if (option) {
          setOverViewStore({ viewMode: option.value });
        }
      }}
      isOptionDisabled={(option) => Boolean(option.disabled)}
      isSearchable={false}
    />
  );
};
