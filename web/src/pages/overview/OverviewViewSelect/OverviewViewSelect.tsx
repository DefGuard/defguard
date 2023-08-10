import { useEffect, useMemo } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../i18n/i18n-react';
import { deviceBreakpoints } from '../../../shared/constants';
import { Select } from '../../../shared/defguard-ui/components/Layout/Select/Select';
import { SelectOption } from '../../../shared/defguard-ui/components/Layout/Select/types';
import { OverviewLayoutType } from '../../../shared/types';
import { useOverviewStore } from '../hooks/store/useOverviewStore';

export const OverviewViewSelect = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const defaultViewMode = useOverviewStore((state) => state.defaultViewMode);
  const viewMode = useOverviewStore((state) => state.viewMode);
  const setOverViewStore = useOverviewStore((state) => state.setState);
  const { LL } = useI18nContext();

  useEffect(() => {
    setOverViewStore({ viewMode: defaultViewMode });
  }, [defaultViewMode, setOverViewStore]);

  const getSelectOptions = useMemo((): SelectOption<OverviewLayoutType>[] => {
    if (breakpoint === 'mobile') {
      return [
        {
          key: 0,
          value: OverviewLayoutType.GRID,
          label: LL.networkOverview.filterLabels.grid(),
        },
        {
          key: 1,
          value: OverviewLayoutType.LIST,
          label: LL.networkOverview.filterLabels.list(),
          disabled: true,
        },
      ];
    }
    if (breakpoint === 'tablet') {
      return [
        {
          key: 0,
          value: OverviewLayoutType.GRID,
          label: LL.networkOverview.filterLabels.grid(),
          disabled: true,
        },
        {
          key: 1,
          value: OverviewLayoutType.LIST,
          label: LL.networkOverview.filterLabels.list(),
          disabled: false,
        },
      ];
    }
    return [
      {
        key: 0,
        value: OverviewLayoutType.GRID,
        label: LL.networkOverview.filterLabels.grid(),
      },
      {
        key: 1,
        value: OverviewLayoutType.LIST,
        label: LL.networkOverview.filterLabels.list(),
      },
    ];
  }, [LL.networkOverview.filterLabels, breakpoint]);

  return (
    <Select
      options={getSelectOptions}
      selected={viewMode}
      searchable={false}
      disabled={false}
      loading={false}
      onChangeSingle={(mode) => setOverViewStore({ viewMode: mode })}
    />
  );
};
