import { useEffect, useMemo } from 'react';
import useBreakpoint from 'use-breakpoint';

import {
  Select,
  SelectOption,
} from '../../../shared/components/layout/Select/Select';
import { deviceBreakpoints } from '../../../shared/constants';
import { OverviewLayoutType } from '../../../shared/types';
import { useOverviewStore } from '../hooks/store/useOverviewStore';

export const OverviewViewSelect = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const defaultViewMode = useOverviewStore((state) => state.defaultViewMode);
  const viewMode = useOverviewStore((state) => state.viewMode);
  const setOverViewStore = useOverviewStore((state) => state.setState);

  useEffect(() => {
    setOverViewStore({ viewMode: defaultViewMode });
  }, [defaultViewMode, setOverViewStore]);

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
          key: 2,
          value: OverviewLayoutType.MAP,
          label: 'Map view',
          disabled: true,
        },*/
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
          key: 2,
          value: OverviewLayoutType.MAP,
          label: 'Map view',
          disabled: true,
        },*/
      ];
    }
    return [
      { key: 0, value: OverviewLayoutType.GRID, label: 'Grid view' },
      { key: 1, value: OverviewLayoutType.LIST, label: 'List view' },
      /*{
        key: 2,
        value: OverviewLayoutType.MAP,
        label: 'Map view',
        disabled: true,
      },*/
    ];
  }, [breakpoint]);

  const getSelectValue = useMemo(() => {
    return getSelectOptions.find((o) => o.value === viewMode);
  }, [getSelectOptions, viewMode]);

  return (
    <Select
      options={getSelectOptions}
      selected={getSelectValue}
      searchable={false}
      disabled={false}
      multi={false}
      loading={false}
      onChange={(option) => {
        if (option) {
          setOverViewStore({
            viewMode: (option as SelectOption<OverviewLayoutType>).value,
          });
        }
      }}
    />
  );
};
