import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import {
  getAppliedAliasesQueryOptions,
  getAppliedDestinationsQueryOptions,
  getGroupsInfoQueryOptions,
  getLicenseInfoQueryOptions,
  getLocationsQueryOptions,
  getNetworkDevicesQueryOptions,
  getUsersQueryOptions,
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
  const { data: groups, isLoading: groupsLoading } = useQuery(getGroupsInfoQueryOptions);
  const { data: locations, isLoading: locationsLoading } = useQuery(
    getLocationsQueryOptions,
  );
  const { data: users, isLoading: usersLoading } = useQuery(getUsersQueryOptions);
  const { data: devices, isLoading: devicesLoading } = useQuery(
    getNetworkDevicesQueryOptions,
  );

  const destinationsById = useMemo(() => resourceById(destinations), [destinations]);
  const aliasesById = useMemo(() => resourceById(aliases), [aliases]);
  const groupsById = useMemo(() => resourceById(groups), [groups]);
  const locationsById = useMemo(() => resourceById(locations), [locations]);
  const usersById = useMemo(() => resourceById(users), [users]);
  const devicesById = useMemo(() => resourceById(devices), [devices]);

  return {
    loading:
      aliasesLoading ||
      groupsLoading ||
      locationsLoading ||
      usersLoading ||
      devicesLoading ||
      destinationsLoading ||
      licenseLoading,
    aliases: aliasesById,
    groups: groupsById,
    locations: locationsById,
    users: usersById,
    devices: devicesById,
    destinations: destinationsById,
    license: licenseInfo,
  };
};
