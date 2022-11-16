import { useMemo } from 'react';
import useBreakpoint from 'use-breakpoint';

import {
  Select,
  SelectOption,
} from '../../../shared/components/layout/Select/Select';
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
          key: 0,
          value: OverviewLayoutType.GRID,
          label: 'Grid view',
        },
        {
          key: 1,
          value: OverviewLayoutType.LIST,
          label: 'List view',
          disabled: true,
        },
        /*{
        //key: 2,
        //value: OverviewLayoutType.MAP,
        //label: 'Map view',
        //disabled: true,
				}*/
      ];
    }
    if (breakpoint === 'tablet') {
      return [
        {
          key: 0,
          value: OverviewLayoutType.GRID,
          label: 'Grid view',
          disabled: true,
        },
        {
          key: 1,
          value: OverviewLayoutType.LIST,
          label: 'List view',
          disabled: false,
        },
        /*{
        //key: 2,
        //value: OverviewLayoutType.MAP,
        //label: 'Map view',
        //disabled: true,
      },*/
      ];
    }
    return [
      { key: 0, value: OverviewLayoutType.GRID, label: 'Grid view' },
      { key: 1, value: OverviewLayoutType.LIST, label: 'List view' },
      /*{
        //key: 2,
        //value: OverviewLayoutType.MAP,
        //label: 'Map view',
        //disabled: true,
      },*/
    ];
  }, [breakpoint]);

  const getSelectValue = useMemo(() => {
    return getSelectOptions.find((o) => o.value === defaultViewMode);
  }, [getSelectOptions, defaultViewMode]);

  return (
    <Select
      options={getSelectOptions}
      selected={getSelectValue}
      onChange={(option) => {
        if (option) {
          setOverViewStore({
            defaultViewMode: (option as SelectOption<OverviewLayoutType>).value,
            viewMode: (option as SelectOption<OverviewLayoutType>).value,
          });
        }
      }}
      searchable={false}
      disabled={false}
      multi={false}
      loading={false}
    />
  );
};
