import { useMemo } from 'react';
import Select from 'react-select';
import useBreakpoint from 'use-breakpoint';

import { deviceBreakpoints } from '../../../shared/constants';
import { OverviewLayoutType } from '../../../shared/types';
import { useOverviewStore } from '../../overview/hooks/store/useOverviewStore';

export const DefaultNetworkSelect = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const defaultViewMode = useOverviewStore((state) => state.defaultViewMode);
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
    return getSelectOptions.find((o) => o.value === defaultViewMode);
  }, [getSelectOptions, defaultViewMode]);

  return (
    <Select
      options={getSelectOptions}
      className="custom-select"
      classNamePrefix="rs"
      value={getSelectValue}
      onChange={(option) => {
        if (option) {
          setOverViewStore({
            defaultViewMode: option.value,
            viewMode: option.value,
          });
        }
      }}
      isOptionDisabled={(option) => Boolean(option.disabled)}
      isSearchable={false}
    />
  );
};
