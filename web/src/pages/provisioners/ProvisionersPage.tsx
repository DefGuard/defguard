import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { orderBy } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { deviceBreakpoints } from '../../shared/constants';
import { LoaderSpinner } from '../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import NoData from '../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Select } from '../../shared/defguard-ui/components/Layout/Select/Select';
import { SelectOption } from '../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { ProvisionersList } from './ProvisionersList/ProvisionersList';
import { ProvisioningStationSetup } from './ProvisioningStationSetup';
import { Search } from '../../shared/defguard-ui/components/Layout/Search/Search';

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
    [LL.provisionersOverview.filterLabels],
  );

  const [selectedFilterOption, setSelectedFilterOption] = useState(FilterOptions.ALL);

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
    },
  );

  const filteredProvisioners = useMemo(() => {
    let res = orderBy(provisioners, ['id'], ['desc']);
    res = res.filter((p) => p.id.toLowerCase().includes(searchValue.toLowerCase()));
    switch (selectedFilterOption) {
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
  }, [provisioners, searchValue, selectedFilterOption]);

  useEffect(() => {
    if (breakpoint !== 'desktop' && selectedFilterOption === FilterOptions.ALL) {
      setSelectedFilterOption(FilterOptions.ALL);
    }
  }, [breakpoint, filterSelectOptions, selectedFilterOption]);

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
              searchable={false}
              onChangeSingle={(filter) => setSelectedFilterOption(filter)}
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
