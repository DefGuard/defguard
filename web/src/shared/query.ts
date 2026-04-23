import { queryOptions } from '@tanstack/react-query';
import api from './api/api';
import { AclDeploymentState, type UserProfile } from './api/types';
import { updateServiceApi, updateServiceClient } from './api/update-service';
import {
  contextualHelpPath,
  parseContextualHelp,
} from './components/ContextualHelp/data';
import { resourceDisplayMap } from './utils/resourceById';
import { parseVideoTutorials, videoTutorialsPath } from './video-tutorials/data';

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

export const getLocationsCountQueryOptions = queryOptions({
  queryFn: api.location.getCount,
  queryKey: ['network', 'count'],
  select: (resp) => resp.data.count,
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

export const getLocationsDisplayQueryOptions = queryOptions({
  queryFn: api.location.getLocationsDisplay,
  queryKey: ['network', 'display', 'resourceMap'],
  select: (resp) => resourceDisplayMap(resp.data),
});

export const getEdgesQueryOptions = queryOptions({
  queryFn: api.edge.getEdges,
  queryKey: ['edge'],
  select: (resp) => resp.data,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getGatewaysQueryOptions = queryOptions({
  queryFn: api.gateway.getGateways,
  queryKey: ['gateway'],
  select: (resp) => resp.data,
  refetchInterval: 30_000,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getEdgeQueryOptions = (id: number) =>
  queryOptions({
    queryFn: () => api.edge.getEdge(id),
    queryKey: ['edge', id],
    select: (resp) => resp.data,
  });

export const getGatewayQueryOptions = (id: number) =>
  queryOptions({
    queryFn: () => api.gateway.getGateway(id),
    queryKey: ['gateway', id],
    select: (resp) => resp.data,
  });

export const getNetworkDevicesQueryOptions = queryOptions({
  queryFn: api.network_device.getDevices,
  queryKey: ['device', 'network'],
});

export const getUserMeQueryOptions = queryOptions({
  queryFn: api.user.getMe,
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
        devices: data.user.devices.map((device) => ({
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

export const videoTutorialsQueryOptions = queryOptions({
  queryKey: ['update-service', 'video-tutorials'],
  queryFn: () => updateServiceClient.get<unknown>(videoTutorialsPath),
  select: (resp) => {
    try {
      return parseVideoTutorials(resp.data);
    } catch (err) {
      console.error(
        '[video-tutorials] Fetched successfully but failed to parse response:',
        err,
      );
      throw err;
    }
  },
  // Mappings are version-tied and won't meaningfully change within a session.
  staleTime: Infinity,
  // Silent failure: if the fetch or parse fails, the widget simply won't appear.
  retry: false,
});

export const contextualHelpQueryOptions = queryOptions({
  queryKey: ['update-service', 'contextual-help'],
  queryFn: () => updateServiceClient.get<unknown>(contextualHelpPath),
  select: (resp) => {
    try {
      return parseContextualHelp(resp.data);
    } catch (err) {
      console.error(
        '[contextual-help] Fetched successfully but failed to parse response:',
        err,
      );
      throw err;
    }
  },
  staleTime: Infinity,
  retry: false,
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
});

export const getUsersOverviewQueryOptions = queryOptions({
  queryFn: api.user.getUsers,
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

export const getAliasesCountQueryOptions = queryOptions({
  queryFn: api.acl.alias.getCount,
  queryKey: ['acl', 'alias', 'count'],
  select: (resp) => resp.data,
});
export const getDestinationsCountQueryOptions = queryOptions({
  queryFn: api.acl.destination.getCount,
  queryKey: ['acl', 'destination', 'count'],
  select: (resp) => resp.data,
});

export const getRulesCountQueryOptions = queryOptions({
  queryFn: api.acl.rule.getCount,
  queryKey: ['acl', 'rule', 'count'],
  select: (resp) => resp.data,
});

export const getRulesQueryOptions = queryOptions({
  queryFn: api.acl.rule.getRules,
  queryKey: ['acl', 'rule'],
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
});

export const getActivityLogStreamsQueryOptions = queryOptions({
  queryFn: api.activityLogStream.getStreams,
  queryKey: ['activity_log_stream'],
  select: (resp) => resp.data,
});

export const getSessionInfoQueryOptions = queryOptions({
  queryFn: api.getSessionInfo,
  queryKey: ['session-info'],
  select: (resp) => resp.data,
  refetchOnMount: true,
  refetchOnReconnect: true,
  refetchOnWindowFocus: false,
});

export const getVersionQueryOptions = queryOptions({
  queryFn: api.app.version,
  queryKey: ['version'],
  select: (resp) => resp.data.version,
  refetchOnMount: true,
  refetchOnReconnect: true,
  refetchOnWindowFocus: false,
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

export const getInitialSetupCaQueryOptions = queryOptions({
  queryFn: api.initial_setup.getCA,
  queryKey: ['initial_setup', 'ca'],
  select: (resp) => resp.data,
});

export const getMigrationCaQueryOptions = queryOptions({
  queryFn: api.migration.ca.getCA,
  queryKey: ['migration', 'ca'],
  select: (resp) => resp.data,
});

export const getMigrationStateQueryOptions = queryOptions({
  queryFn: api.migration.state.getMigrationState,
  queryKey: ['migration', 'state'],
  select: (resp) => resp.data,
});
