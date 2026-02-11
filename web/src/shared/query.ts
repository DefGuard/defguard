import { queryOptions } from '@tanstack/react-query';
import api from './api/api';
import { AclDeploymentState, type UserProfile } from './api/types';
import { updateServiceApi } from './api/update-service';

export const getExternalProviderQueryOptions = queryOptions({
  queryFn: api.openIdProvider.getOpenIdProvider,
  queryKey: ['openid', 'provider'],
  select: (resp) => resp.data,
});

export const getEnterpriseSettingsQueryOptions = queryOptions({
  queryFn: api.settings.getEnterpriseSettings,
  queryKey: ['settings_enterprise'],
  select: (resp) => resp.data,
});

export const getLocationQueryOptions = (id: number) =>
  queryOptions({
    queryFn: () => api.location.getLocation(id),
    queryKey: ['network', id],
    select: (resp) => resp.data,
  });

export const getLocationsQueryOptions = queryOptions({
  queryFn: api.location.getLocations,
  queryKey: ['network'],
  select: (resp) => resp.data,
});

export const getEdgesQueryOptions = queryOptions({
  queryFn: api.edge.getEdges,
  queryKey: ['edge'],
  select: (resp) => resp.data,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getEdgeQueryOptions = (id: number) =>
  queryOptions({
    queryFn: () => api.edge.getEdge(id),
    queryKey: ['edge', id],
    select: (resp) => resp.data,
  });

export const getNetworkDevicesQueryOptions = queryOptions({
  queryFn: api.network_device.getDevices,
  queryKey: ['device', 'network'],
  select: (resp) => resp.data,
});

export const getUserMeQueryOptions = queryOptions({
  queryFn: () => api.user.getMe,
  queryKey: ['me'],
  staleTime: 60_000,
  throwOnError: false,
  retry: false,
  refetchOnWindowFocus: false,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const userProfileQueryOptions = (username: string) =>
  queryOptions({
    queryFn: () => api.user.getUser(username),
    select: ({ data }) => {
      const res: UserProfile = {
        devices: data.devices.map((device) => ({
          ...device,
          biometry_enabled: data.biometric_enabled_devices.includes(device.id),
        })),
        security_keys: data.security_keys,
        user: data.user,
      };
      return res;
    },
    queryKey: ['user', username],
    refetchOnMount: true,
    refetchOnReconnect: true,
  });

export const clientArtifactsQueryOptions = queryOptions({
  queryFn: updateServiceApi.getClientArtifacts,
  queryKey: ['update-service', 'artifacts'],
  staleTime: 180 * 1000,
  refetchOnWindowFocus: false,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getUserAuthKeysQueryOptions = (username: string) =>
  queryOptions({
    queryFn: () => api.user.getAuthKeys(username),
    queryKey: ['user', username, 'auth_key'],
    select: (response) => response.data,
    refetchOnMount: true,
    refetchOnReconnect: true,
  });

export const getUserApiTokensQueryOptions = (username: string, admin: boolean) =>
  queryOptions({
    queryFn: () => api.user.getApiTokens(username),
    queryKey: ['user', username, 'api_token'],
    select: (resp) => resp.data,
    refetchOnMount: true,
    refetchOnReconnect: true,
    throwOnError: false,
    enabled: admin,
  });

export const getUsersQueryOptions = queryOptions({
  queryFn: api.user.getUsers,
  queryKey: ['user'],
  refetchOnMount: true,
  refetchOnReconnect: true,
  select: (resp) => resp.data,
});

export const getUsersOverviewQueryOptions = queryOptions({
  queryFn: api.getUsersOverview,
  queryKey: ['user-overview'],
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getGroupsInfoQueryOptions = queryOptions({
  queryFn: api.group.getGroupsInfo,
  queryKey: ['group-info'],
  select: (resp) => resp.data,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getOpenIdClientQueryOptions = queryOptions({
  queryFn: api.openIdClient.getOpenIdClients,
  queryKey: ['oauth'],
  select: (resp) => resp.data,
});

export const getWebhooksQueryOptions = queryOptions({
  queryFn: api.webhook.getWebhooks,
  queryKey: ['webhook'],
  select: (resp) => resp.data,
});

export const getSettingsQueryOptions = queryOptions({
  queryFn: api.settings.getSettings,
  queryKey: ['settings'],
  select: (resp) => resp.data,
});

export const getOpenIdProvidersQueryOptions = queryOptions({
  queryFn: api.openIdProvider.getOpenIdProvider,
  queryKey: ['openid', 'provider'],
  select: (resp) => resp.data,
});

export const getRulesQueryOptions = queryOptions({
  queryFn: api.acl.rule.getRules,
  queryKey: ['acl', 'rule'],
  select: (resp) => resp.data,
});

export const getAliasesQueryOptions = queryOptions({
  queryFn: api.acl.alias.getAliases,
  queryKey: ['acl', 'alias'],
  select: (resp) => resp.data,
});

export const getAppliedAliasesQueryOptions = queryOptions({
  queryFn: api.acl.alias.getAliases,
  queryKey: ['acl', 'alias'],
  select: (resp) =>
    resp.data.filter((alias) => alias.state === AclDeploymentState.Applied),
});

export const getDestinationsQueryOptions = queryOptions({
  queryFn: api.acl.destination.getDestinations,
  queryKey: ['acl', 'destination'],
  select: (resp) => resp.data,
});

export const getAppliedDestinationsQueryOptions = queryOptions({
  queryFn: api.acl.destination.getDestinations,
  queryKey: ['acl', 'destination'],
  select: (resp) =>
    resp.data.filter((destination) => destination.state === AclDeploymentState.Applied),
});

export const getLicenseInfoQueryOptions = queryOptions({
  queryFn: api.getLicenseInfo,
  queryKey: ['enterprise_info'],
  select: (response) => response.data.license_info,
});

export const getActivityLogStreamsQueryOptions = queryOptions({
  queryFn: api.activityLogStream.getStreams,
  queryKey: ['activity_log_stream'],
  select: (resp) => resp.data,
});

export const getSettingsEssentialsQueryOptions = queryOptions({
  queryFn: api.settings.getSettingsEssentials,
  queryKey: ['settings_essentials'],
  retry: false,
  refetchOnWindowFocus: false,
  refetchOnMount: true,
  refetchOnReconnect: true,
  staleTime: 60_000,
  select: (resp) => resp.data,
});
