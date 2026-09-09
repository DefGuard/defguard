import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import {
  getAppliedAliasesQueryOptions,
  getAppliedDestinationsQueryOptions,
  getLicenseInfoQueryOptions,
  getLocationsQueryOptions,
} from '../../shared/query';
import { resourceById } from '../../shared/utils/resourceById';

export const useRuleDeps = () => {
  const { data: licenseInfo, isLoading: licenseLoading } = useQuery(
    getLicenseInfoQueryOptions,
  );
  const { data: aliases, isLoading: aliasesLoading } = useQuery(
    getAppliedAliasesQueryOptions,
  );
  const { data: destinations, isLoading: destinationsLoading } = useQuery(
    getAppliedDestinationsQueryOptions,
  );
  const { data: locations, isLoading: locationsLoading } = useQuery(
    getLocationsQueryOptions,
  );

  const destinationsById = useMemo(() => resourceById(destinations), [destinations]);
  const aliasesById = useMemo(() => resourceById(aliases), [aliases]);
  const locationsById = useMemo(() => resourceById(locations), [locations]);

  return {
    loading: aliasesLoading || locationsLoading || destinationsLoading || licenseLoading,
    aliases: aliasesById,
    locations: locationsById,
    destinations: destinationsById,
    license: licenseInfo,
  };
};
