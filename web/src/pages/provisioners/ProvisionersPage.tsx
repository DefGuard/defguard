import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { orderBy } from 'lodash-es';
import { useMemo, useState } from 'react';

import LoaderSpinner from '../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import NoData from '../../shared/components/layout/NoData/NoData';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { Search } from '../../shared/components/layout/Search/Search';
import {
  Select,
  SelectOption,
} from '../../shared/components/layout/Select/Select';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { ProvisionersList } from './ProvisionersList/ProvisionersList';
import { ProvisioningStationSetup } from './ProvisioningStationSetup';

const ProvisionersPage = () => {
  const [selectedFilterOption, setSelectedFilterOption] = useState(
    filterSelectOptions[0]
  );
  const [searchValue, setSearchValue] = useState<string>('');

  const {
    provisioning: { getWorkers },
  } = useApi();

  const license = useAppStore((state) => state.license);

  const hasAccess = useMemo(() => {
    if (!license) {
      return false;
    }
    return license.enterprise || license.worker;
  }, [license]);

  const { data: provisioners, isLoading } = useQuery(
    [QueryKeys.FETCH_WORKERS],
    getWorkers,
    { enabled: hasAccess, refetchOnWindowFocus: false, refetchInterval: 10000 }
  );

  const filteredProvisioners = useMemo(() => {
    let res = orderBy(provisioners, ['id'], ['desc']);
    res = res.filter((p) =>
      p.id.toLowerCase().includes(searchValue.toLowerCase())
    );
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

  return (
    <PageContainer id="provisioners-page">
      <header>
        <h1>Provisioners</h1>
        <Search
          placeholder="Find provisioners"
          initialValue={searchValue}
          debounceTiming={500}
          onDebounce={(val) => setSearchValue(val)}
        />
      </header>
      <div className="provisioners-container">
        <div className="top">
          <div className="provisioners-count">
            <span>All provisioners</span>
            <div className="count">
              <span>{provisioners?.length ?? 0}</span>
            </div>
          </div>
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
        </div>
        {!isLoading &&
          hasAccess &&
          filteredProvisioners &&
          filteredProvisioners.length > 0 && (
            <ProvisionersList provisioners={filteredProvisioners} />
          )}
        {(hasAccess && !filteredProvisioners) ||
        filteredProvisioners.length === 0 ? (
          <NoData customMessage="No provisioners found" />
        ) : null}
        {!hasAccess && <NoData customMessage="No license for this feature" />}
        {isLoading && <LoaderSpinner size={200} />}
      </div>
      <ProvisioningStationSetup hasAccess={hasAccess} />
    </PageContainer>
  );
};

enum FilterOptions {
  ALL = 'all',
  AVAILABLE = 'available',
  UNAVAILABLE = 'unavailable',
}

const filterSelectOptions: SelectOption<FilterOptions>[] = [
  {
    key: 1,
    label: 'All',
    value: FilterOptions.ALL,
  },
  {
    key: 2,
    label: 'Available',
    value: FilterOptions.AVAILABLE,
  },
  {
    key: 3,
    label: 'Unavailable',
    value: FilterOptions.UNAVAILABLE,
  },
];

export default ProvisionersPage;
