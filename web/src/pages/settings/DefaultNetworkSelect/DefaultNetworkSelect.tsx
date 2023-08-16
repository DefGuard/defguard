import './style.scss';

import parse from 'html-react-parser';
import { useCallback, useMemo } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../i18n/i18n-react';
import { deviceBreakpoints } from '../../../shared/constants';
import { Helper } from '../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { Select } from '../../../shared/defguard-ui/components/Layout/Select/Select';
import { SelectSelectedValue } from '../../../shared/defguard-ui/components/Layout/Select/types';
import { externalLink } from '../../../shared/links';
import { OverviewLayoutType } from '../../../shared/types';
import { useOverviewStore } from '../../overview/hooks/store/useOverviewStore';

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
          label: LL.settingsPage.defaultNetworkSelect.filterLabels.grid(),
        },
        {
          key: 1,
          value: OverviewLayoutType.LIST,
          label: LL.settingsPage.defaultNetworkSelect.filterLabels.list(),
          disabled: true,
        },
      ];
    }
    if (breakpoint === 'tablet') {
      return [
        {
          key: 0,
          value: OverviewLayoutType.GRID,
          label: LL.settingsPage.defaultNetworkSelect.filterLabels.grid(),
          disabled: true,
        },
        {
          key: 1,
          value: OverviewLayoutType.LIST,
          label: LL.settingsPage.defaultNetworkSelect.filterLabels.list(),
          disabled: false,
        },
      ];
    }
    return [
      {
        key: 0,
        value: OverviewLayoutType.GRID,
        label: LL.settingsPage.defaultNetworkSelect.filterLabels.grid(),
      },
      {
        key: 1,
        value: OverviewLayoutType.LIST,
        label: LL.settingsPage.defaultNetworkSelect.filterLabels.list(),
      },
    ];
  }, [LL.settingsPage.defaultNetworkSelect.filterLabels, breakpoint]);

  const renderSelected = useCallback(
    (selected: OverviewLayoutType): SelectSelectedValue => {
      const option = getSelectOptions.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exist");
      return {
        key: selected,
        displayValue: option.label,
      };
    },
    [getSelectOptions],
  );

  return (
    <section className="network-view">
      <header>
        <h2>{LL.settingsPage.defaultNetworkSelect.header()}</h2>
        <Helper>
          {parse(
            LL.settingsPage.defaultNetworkSelect.helper({
              documentationLink: externalLink.gitbook.base,
            }),
          )}
        </Helper>
      </header>
      <Select
        searchable={false}
        disabled={false}
        loading={false}
        options={getSelectOptions}
        selected={defaultViewMode}
        renderSelected={renderSelected}
        onChangeSingle={(res) =>
          setOverViewStore({ defaultViewMode: res, viewMode: res })
        }
      />
    </section>
  );
};
