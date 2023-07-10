import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { orderBy } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../i18n/i18n-react';
import { LoaderSpinner } from '../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import NoData from '../../shared/components/layout/NoData/NoData';
import { PageContainer } from '../../shared/components/layout/PageContainer/PageContainer';
import { Search } from '../../shared/components/layout/Search/Search';
import { Select, SelectOption } from '../../shared/components/layout/Select/Select';
import { deviceBreakpoints } from '../../shared/constants';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { ProvisionersList } from './ProvisionersList/ProvisionersList';
import { ProvisioningStationSetup } from './ProvisioningStationSetup';

export const ProvisionersPage = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const { LL } = useI18nContext();
  const filterSelectOptions: SelectOption<FilterOptions>[] = useMemo(
    () => [
      {
        key: 1,
        label: LL.provisionersOverview.filterLabels.all(),
        value: FilterOptions.ALL,
      },
      {
        key: 2,
        label: LL.provisionersOverview.filterLabels.available(),
        value: FilterOptions.AVAILABLE,
      },
      {
        key: 3,
        label: LL.provisionersOverview.filterLabels.unavailable(),
        value: FilterOptions.UNAVAILABLE,
      },
    ],
    [LL.provisionersOverview.filterLabels]
  );

  const [selectedFilterOption, setSelectedFilterOption] = useState(
    filterSelectOptions[0]
  );
  const [searchValue, setSearchValue] = useState<string>('');

  const {
    provisioning: { getWorkers },
  } = useApi();

  const { data: provisioners, isLoading } = useQuery(
    [QueryKeys.FETCH_WORKERS],
    getWorkers,
    {
      refetchOnWindowFocus: false,
      refetchInterval: 5000,
    }
  );

  const filteredProvisioners = useMemo(() => {
    let res = orderBy(provisioners, ['id'], ['desc']);
    res = res.filter((p) => p.id.toLowerCase().includes(searchValue.toLowerCase()));
    switch (selectedFilterOption.value) {
      case FilterOptions.ALL:
        break;
      case FilterOptions.AVAILABLE:
        res = res.filter((p) => p.connected === true);
        break;
      case FilterOptions.UNAVAILABLE:
        res = res.filter((p) => p.connected === false);
        break;
    }
    return res;
  }, [provisioners, searchValue, selectedFilterOption.value]);

  useEffect(() => {
    if (breakpoint !== 'desktop' && selectedFilterOption.value === FilterOptions.ALL) {
      setSelectedFilterOption(filterSelectOptions[0]);
    }
  }, [breakpoint, filterSelectOptions, selectedFilterOption.value]);

  return (
    <PageContainer id="provisioners-page">
      <header>
        <h1>{LL.provisionersOverview.pageTitle()}</h1>
        <Search
          placeholder={LL.provisionersOverview.search.placeholder()}
          initialValue={searchValue}
          debounceTiming={500}
          onDebounce={(val) => setSearchValue(val)}
        />
      </header>
      <div className="provisioners-container">
        <div className="top">
          <div className="provisioners-count">
            <span>{LL.provisionersOverview.provisionersCount()}</span>
            <div className="count">
              <span>{provisioners?.length ?? 0}</span>
            </div>
          </div>
          {breakpoint === 'desktop' && (
            <Select
              options={filterSelectOptions}
              selected={selectedFilterOption}
              multi={false}
              searchable={false}
              onChange={(val) => {
                if (val && !Array.isArray(val)) {
                  setSelectedFilterOption(val);
                }
              }}
            />
          )}
        </div>
        {!isLoading && filteredProvisioners && filteredProvisioners.length > 0 && (
          <ProvisionersList provisioners={filteredProvisioners} />
        )}
        {!isLoading &&
          (!filteredProvisioners || !filteredProvisioners.length ? (
            <NoData customMessage={LL.provisionersOverview.noProvisionersFound()} />
          ) : null)}
        {isLoading && (
          <div className="loader">
            <LoaderSpinner size={130} />
          </div>
        )}
      </div>
      <div className="setup-container">
        <ProvisioningStationSetup />
      </div>
    </PageContainer>
  );
};

enum FilterOptions {
  ALL = 'all',
  AVAILABLE = 'available',
  UNAVAILABLE = 'unavailable',
}
