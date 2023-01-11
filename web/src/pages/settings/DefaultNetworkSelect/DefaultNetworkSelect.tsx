import './style.scss';

import { useMemo } from 'react';
import useBreakpoint from 'use-breakpoint';

import { Helper } from '../../../shared/components/layout/Helper/Helper';
import {
  Select,
  SelectOption,
  SelectStyleVariant,
} from '../../../shared/components/layout/Select/Select';
import { deviceBreakpoints } from '../../../shared/constants';
import { OverviewLayoutType } from '../../../shared/types';
import { useOverviewStore } from '../../overview/hooks/store/useOverviewStore';
import { useI18nContext } from '../../../i18n/i18n-react';
import parse from 'html-react-parser';

export const DefaultNetworkSelect = () => {
  const { LL } = useI18nContext();
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
      ];
    }
    return [
      { key: 0, value: OverviewLayoutType.GRID, label: 'Grid view' },
      { key: 1, value: OverviewLayoutType.LIST, label: 'List view' },
    ];
  }, [breakpoint]);

  const getSelectValue = useMemo(() => {
    return getSelectOptions.find((o) => o.value === defaultViewMode);
  }, [getSelectOptions, defaultViewMode]);

  return (
    <section className="network-view">
      <header>
        <h2>{LL.settingsPage.defaultNetworkSelect.header()}</h2>
        <Helper>{parse(LL.settingsPage.defaultNetworkSelect.helper())}</Helper>
      </header>
      <Select
        styleVariant={SelectStyleVariant.WHITE}
        options={getSelectOptions}
        selected={getSelectValue}
        onChange={(option) => {
          if (option) {
            setOverViewStore({
              defaultViewMode: (option as SelectOption<OverviewLayoutType>)
                .value,
              viewMode: (option as SelectOption<OverviewLayoutType>).value,
            });
          }
        }}
        searchable={false}
        disabled={false}
        multi={false}
        loading={false}
      />
    </section>
  );
};
