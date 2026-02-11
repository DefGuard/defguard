import dayjs from 'dayjs';
import { cloneDeep } from 'lodash-es';
import { removeEmptyStrings } from '../utils/removeEmptyStrings';
import { client } from './api-client';
import type {
  AclAlias,
  AclDestination,
  AclRule,
  ActivityLogEvent,
  ActivityLogRequestParams,
  ActivityLogStream,
  AddAclAliasRequest,
  AddAclDestination,
  AddAclRuleRequest,
  AddApiTokenRequest,
  AddApiTokenResponse,
  AddAuthKeyRequest,
  AddDeviceRequest,
  AddDeviceResponse,
  AddDeviceResponseConfig,
  AddNetworkDeviceRequest,
  AddNetworkDeviceResponse,
  AddOpenIdClient,
  AddOpenIdProvider,
  AddUserRequest,
  AddUsersToGroupsRequest,
  AddWebhookRequest,
  AdminChangeUserPasswordRequest,
  ApiToken,
  ApplicationInfo,
  AuthKey,
  AvailableLocationIpResponse,
  ChangeAccountActiveRequest,
  ChangeWebhookStateRequest,
  CreateActivityLogStreamRequest,
  CreateAdminRequest,
  CreateCARequest,
  CreateGroupRequest,
  DeleteApiTokenRequest,
  DeleteAuthKeyRequest,
  DeleteGatewayRequest,
  Device,
  Edge,
  EdgeInfo,
  EditAclAliasRequest,
  EditAclDestination,
  EditAclRuleRequest,
  EditGroupRequest,
  EditNetworkDeviceRequest,
  EditNetworkLocation,
  EditNetworkLocationRequest,
  EditOpenIdClientActiveStateRequest,
  EnableMfaMethodResponse,
  GatewayStatus,
  GetCAResponse,
  GroupInfo,
  GroupsResponse,
  IpValidation,
  LicenseInfoResponse,
  LocationDevicesStats,
  LocationStats,
  LocationStatsRequest,
  LoginRequest,
  LoginResponse,
  LoginResponseBasic,
  MfaCompleteResponse,
  NetworkDevice,
  NetworkLocation,
  OpenIdAuthInfo,
  OpenIdClient,
  OpenIdProvidersResponse,
  PaginatedResponse,
  RenameApiTokenRequest,
  RenameAuthKeyRequest,
  SetGeneralConfigRequest,
  Settings,
  SettingsEnterprise,
  SettingsEssentials,
  StartEnrollmentRequest,
  StartEnrollmentResponse,
  TestDirectorySyncResponse,
  TotpInitResponse,
  UploadCARequest,
  User,
  UserChangePasswordRequest,
  UserDevice,
  UserProfileResponse,
  UsersListItem,
  ValidateDeviceIpsRequest,
  WebauthnLoginStartResponse,
  WebauthnRegisterFinishRequest,
  WebauthnRegisterStartResponse,
  Webhook,
} from './types';

const api = {
  getUsersOverview: async (): Promise<UsersListItem[]> => {
    const { data: users } = await api.user.getUsers();
    const res: UsersListItem[] = [];
    for (const user of users) {
      const { data: profile } = await api.user.getUser(user.username);
      res.push({
        ...user,
        name: `${user.first_name} ${user.last_name}`,
        devices: profile.devices,
      });
    }
    return res;
  },
  initial_setup: {
    createCA: (data: CreateCARequest) => client.post('/initial_setup/ca', data),
    getCA: () => client.get<GetCAResponse>('/initial_setup/ca'),
    uploadCA: (data: UploadCARequest) => client.post('/initial_setup/ca/upload', data),
    createAdminUser: (data: CreateAdminRequest) =>
      client.post('/initial_setup/admin', data),
    login: (data: LoginRequest) => client.post('/initial_setup/login', data),
    session: () => client.get('/initial_setup/session'),
    setGeneralConfig: (data: SetGeneralConfigRequest) =>
      client.post('/initial_setup/general_config', data),
    finishSetup: () => client.post('/initial_setup/finish'),
  },
  openid: {
    authInfo: () => client.get<OpenIdAuthInfo>(`/openid/auth_info`),
    callback: (data: unknown) => client.post<LoginResponse>(`/openid/callback`, data),
  },
  openIdClient: {
    getOpenIdClients: () => client.get<OpenIdClient[]>(`/oauth`),
    getOpenIdClient: (clientId: string) => client.get<OpenIdClient>(`/oauth/${clientId}`),
    addOpenIdClient: (data: AddOpenIdClient) => client.post(`/oauth`, data),
    editOpenIdClient: (data: OpenIdClient) =>
      client.put(`/oauth/${data.client_id}`, data),
    deleteOpenIdClient: (clientId: string) => client.delete(`/oauth/${clientId}`),
    changeOpenIdClientState: (data: EditOpenIdClientActiveStateRequest) =>
      client.post(`/oauth/${data.client_id}`, {
        enabled: data.enabled,
      }),
  },
  group: {
    addGroup: (data: CreateGroupRequest) => client.post('/group', data),
    getGroups: () => client.get<GroupsResponse>('/group'),
    getGroupsInfo: () => client.get<GroupInfo[]>('/group-info'),
    editGroup: ({ originalName, ...data }: EditGroupRequest) =>
      client.put(`/group/${originalName ?? data.name}`, data),
    deleteGroup: (name: string) => client.delete(`/group/${name}`),
    addUsersToGroups: (data: AddUsersToGroupsRequest) =>
      client.post(`/groups-assign`, data),
  },
  app: {
    info: () => client.get<ApplicationInfo>('/info'),
  },
  user: {
    addUser: (data: AddUserRequest) => client.post<User>('/user', data),
    usernameAvailable: (username: string) =>
      client.post('/user/available', {
        username,
      }),
    getMe: () => client.get<User>('/me'),
    getUsers: () => client.get<User[]>('/user'),
    getUser: (username: string) => client.get<UserProfileResponse>(`/user/${username}`),
    editUser: (data: { username: string; body: User }) =>
      client.put(`/user/${data.username}`, data.body),
    changePassword: (data: UserChangePasswordRequest) =>
      client.put(`/user/change_password`, data),
    adminChangePassword: ({ new_password, username }: AdminChangeUserPasswordRequest) =>
      client.put(`/user/${username}/password`, {
        new_password,
      }),
    resetPassword: (username: string) => client.post(`/user/${username}/reset_password`),
    getUserDevices: (username: string) =>
      client.get<UserDevice[]>(`/device/user/${username}`),
    startClientActivation: (data: StartEnrollmentRequest) =>
      client.post<StartEnrollmentResponse>(`/user/${data.username}/start_desktop`, data),
    startEnrollment: (data: StartEnrollmentRequest) =>
      client.post<StartEnrollmentResponse>(
        `/user/${data.username}/start_enrollment`,
        data,
      ),
    getAuthKeys: (username: string) =>
      client.get<AuthKey[]>(`/user/${username}/auth_key`),
    addAuthKey: ({ username, ...data }: AddAuthKeyRequest) =>
      client.post(`/user/${username}/auth_key`, data),
    renameAuthKey: ({ id, username, ...data }: RenameAuthKeyRequest) =>
      client.post(`/user/${username}/auth_key/${id}/rename`, data),
    deleteAuthKey: ({ username, id }: DeleteAuthKeyRequest) =>
      client.delete(`/user/${username}/auth_key/${id}`),
    addApiToken: ({ username, ...data }: AddApiTokenRequest) =>
      client.post<AddApiTokenResponse>(`/user/${username}/api_token`, data),
    getApiTokens: (username: string) =>
      client.get<ApiToken[]>(`/user/${username}/api_token`),
    renameApiToken: ({ username, id, ...data }: RenameApiTokenRequest) =>
      client.post(`/user/${username}/api_token/${id}/rename`, data),
    deleteApiToken: ({ username, id }: DeleteApiTokenRequest) =>
      client.delete(`/user/${username}/api_token/${id}`),
    disableMfa: (username: string) => client.delete(`/user/${username}/mfa`),
    activeStateChange: async ({
      active,
      username,
    }: ChangeAccountActiveRequest): Promise<void> => {
      const { data: profile } = await api.user.getUser(username);
      const clone = removeEmptyStrings(cloneDeep(profile.user));
      clone.is_active = active;
      await api.user.editUser({
        username,
        body: clone,
      });
    },
    deleteUser: (username: string) => client.delete(`/user/${username}`),
    mfa: {
      totp: {
        disable: (username: string) => client.delete(`/user/${username}/totp`),
      },
      email: {
        disable: (username: string) => client.delete(`/user/${username}/email`),
      },
    },
  },
  webhook: {
    addWebhook: (data: AddWebhookRequest) => client.post('/webhook', data),
    deleteWebhook: (id: number) => client.delete(`/webhook/${id}`),
    editWebhook: ({ id, ...rest }: Webhook) => client.put(`/webhook/${id}`, rest),
    changeWebhookState: (data: ChangeWebhookStateRequest) =>
      client.post(`/webhook/${data.id}`, {
        enabled: data.enabled,
      }),
    getWebhook: (id: number) => client.get<Webhook>(`/webhook/${id}`),
    getWebhooks: () => client.get<Webhook[]>(`/webhook`),
  },
  auth: {
    login: (data: LoginRequest) => client.post<LoginResponse>(`/auth`, data),
    logout: () => client.post('/auth/logout'),
    mfa: {
      enable: () => client.put('/auth/mfa'),
      disable: () => client.delete('/auth/mfa'),
      recovery: (code: string) =>
        client.post<MfaCompleteResponse>('/auth/recovery', { code }),
      totp: {
        init: () => client.post<TotpInitResponse>('/auth/totp/init'),
        enable: (code: string) =>
          client.post<EnableMfaMethodResponse>('/auth/totp', {
            code,
          }),
        verify: (code: string) =>
          client.post<MfaCompleteResponse>('/auth/totp/verify', {
            code,
          }),
      },
      email: {
        init: () => client.post('/auth/email/init'),
        enable: (code: string) =>
          client.post<EnableMfaMethodResponse>('/auth/email', {
            code,
          }),
        resend: () => client.get('/auth/email'),
        verify: (code: string) =>
          client.post<MfaCompleteResponse>('/auth/email/verify', { code }),
      },
      webauthn: {
        deleteKey: (data: { username: string; keyId: number | string }) =>
          client.delete(`/user/${data.username}/security_key/${data.keyId}`),
        register: {
          start: (name: string) =>
            client.post<WebauthnRegisterStartResponse>('/auth/webauthn/init', {
              name,
            }),
          finish: (data: WebauthnRegisterFinishRequest) =>
            client.post<EnableMfaMethodResponse>('/auth/webauthn/finish', data),
        },
        login: {
          start: () => client.post<WebauthnLoginStartResponse>('/auth/webauthn/start'),
          finish: (data: PublicKeyCredentialJSON) =>
            client.post<LoginResponseBasic>('/auth/webauthn', data),
        },
      },
    },
  },
  network_device: {
    deleteDevice: (id: number) => client.delete(`/device/network/${id}`),
    editDevice: ({ id, ...data }: EditNetworkDeviceRequest) =>
      client.put(`/device/network/${id}`, data),
    getDevice: (id: number) => client.get<NetworkDevice>(`/device/network/${id}`),
    getDevices: () => client.get<NetworkDevice[]>(`/device/network`),
    getDeviceConfig: (id: number) => client.get<string>(`/device/network/${id}/config`),
    generateToken: (id: number) =>
      client.post<StartEnrollmentResponse>(`/device/network/start_cli/${id}`),
    addCliDevice: (data: AddNetworkDeviceRequest) =>
      client.post<StartEnrollmentResponse>('/device/network/start_cli', data),
    startCliForDevice: (id: number) =>
      client.post<StartEnrollmentResponse>(`/device/network/start_cli/${id}`),
    addDevice: (data: AddNetworkDeviceRequest) =>
      client.post<AddNetworkDeviceResponse>(`/device/network`, data),
    getAvailableIp: (locationId: number) =>
      client.get<AvailableLocationIpResponse>(`/device/network/ip/${locationId}`),
    validateIps: (data: ValidateDeviceIpsRequest) =>
      client.post<IpValidation[]>(`/device/network/ip/${data.locationId}`, {
        ips: data.ips,
      }),
  },
  location: {
    deleteLocation: (locationId: number) => client.delete(`/network/${locationId}`),
    getLocationsSummary: (from?: number) =>
      client.get<LocationStats>(`/network/stats`, {
        params: {
          from: from ? dayjs.utc().subtract(from, 'hour').toISOString() : undefined,
        },
      }),
    getLocations: () => client.get<NetworkLocation[]>('/network'),
    getLocation: (id: number) => client.get<NetworkLocation>(`/network/${id}`),
    getLocationStats: ({ id, ...params }: LocationStatsRequest) =>
      client.get<LocationStats>(`/network/${id}/stats`, {
        params: {
          from: params.from
            ? dayjs.utc().subtract(params.from, 'hour').toISOString()
            : undefined,
        },
      }),
    getLocationGatewaysStatus: (id: number) =>
      client.get<GatewayStatus[]>(`/network/${id}/gateways`),
    deleteGateway: ({ gatewayId, networkId }: DeleteGatewayRequest) =>
      client.delete(`/network/${networkId}/gateways/${gatewayId}`),
    getLocationDevicesStats: ({ id, ...params }: LocationStatsRequest) =>
      client.get<LocationDevicesStats>(`/network/${id}/stats/users`, {
        params: {
          from: params.from
            ? dayjs.utc().subtract(params.from, 'hour').toISOString()
            : undefined,
        },
      }),
    addLocation: (data: EditNetworkLocation) =>
      client.post<NetworkLocation>('/network', data),
    editLocation: ({ id, data }: EditNetworkLocationRequest) =>
      client.put(`/network/${id}`, data),
  },
  device: {
    addDevice: ({ username, ...data }: AddDeviceRequest) =>
      client.post<AddDeviceResponse>(`/device/${username}`, data),
    deleteDevice: (deviceId: number) => client.delete(`/device/${deviceId}`),
    editDevice: (device: Device) => client.put<Device>(`/device/${device.id}`, device),
    getDevice: (deviceId: number) => client.get<Device>(`/device/${deviceId}`),
    getDevices: () => client.get<Device[]>('/device'),
    getDeviceConfig: ({ deviceId, networkId }: { networkId: number; deviceId: number }) =>
      client.get<string>(`/network/${networkId}/device/${deviceId}/config`),
    getDeviceConfigs: async (device: Device): Promise<AddDeviceResponse> => {
      const networkConfigurations: AddDeviceResponseConfig[] = [];
      for (const network of device.networks) {
        const { data: config } = await api.device.getDeviceConfig({
          deviceId: device.id,
          networkId: network.network_id,
        });
        networkConfigurations.push({
          config: config,
          network_id: network.network_id,
          network_name: network.network_name,
        });
      }

      return {
        configs: networkConfigurations,
        device,
      };
    },
  },
  settings: {
    getSettings: () => client.get<Settings>('/settings'),
    editSettings: (data: Settings) => client.put('/settings', data),
    patchSettings: (data: Partial<Settings>) => client.patch('/settings', data),
    getEnterpriseSettings: () => client.get<SettingsEnterprise>('/settings_enterprise'),
    patchEnterpriseSettings: (data: Partial<SettingsEnterprise>) =>
      client.patch('/settings_enterprise', data),
    getSettingsEssentials: () => client.get<SettingsEssentials>('/settings_essentials'),
  },
  openIdProvider: {
    getOpenIdProvider: () => client.get<OpenIdProvidersResponse>('/openid/provider'),
    addOpenIdProvider: (data: AddOpenIdProvider) => client.post('/openid/provider', data),
    deleteOpenIdProvider: (name: string) => client.delete(`/openid/provider/${name}`),
    editOpenIdProvider: (data: AddOpenIdProvider) =>
      client.put(`/openid/provider/${data.name}`, data),
    testDirectorySync: () =>
      client.get<TestDirectorySyncResponse>(`/test_directory_sync`),
  },
  mail: {
    sendTestEmail: (data: { email: string }) => client.post('/mail/test', data),
  },
  edge: {
    getEdges: () => client.get<EdgeInfo[]>('/proxy'),
    getEdge: (edgeId: number | string) => client.get<Edge>(`/proxy/${edgeId}`),
    editEdge: (data: Edge) => client.put(`/proxy/${data.id}`, data),
    deleteEdge: (edgeId: number | string) => client.delete(`/proxy/${edgeId}`),
  },
  acl: {
    destination: {
      getDestinations: () => client.get<AclDestination[]>('/acl/destination'),
      getDestination: (destinationId: number | string) =>
        client.get<AclDestination>(`/acl/destination/${destinationId}`),
      addDestination: (data: AddAclDestination) => client.post(`/acl/destination`, data),
      editDestination: (data: EditAclDestination) =>
        client.put(`/acl/destination/${data.id}`, data),
      deleteDestination: (destinationId: number | string) =>
        client.delete(`/acl/destination/${destinationId}`),
      applyDestinations: (destinations: number[]) =>
        client.put(`/acl/alias/apply`, {
          aliases: destinations,
        }),
    },
    alias: {
      getAliases: () => client.get<AclAlias[]>('/acl/alias'),
      getAlias: (aliasId: number | string) =>
        client.get<AclAlias>(`/acl/alias/${aliasId}`),
      addAlias: (data: AddAclAliasRequest) => client.post(`/acl/alias`, data),
      editAlias: (data: EditAclAliasRequest) => client.put(`/acl/alias/${data.id}`, data),
      deleteAlias: (aliasId: number | string) => client.delete(`/acl/alias/${aliasId}`),
      applyAliases: (aliases: number[]) =>
        client.put(`/acl/alias/apply`, {
          aliases,
        }),
    },
    rule: {
      getRules: () => client.get<AclRule[]>(`/acl/rule`),
      getRule: (ruleId: number | string) => client.get<AclRule>(`/acl/rule/${ruleId}`),
      addRule: (data: AddAclRuleRequest) => client.post(`/acl/rule`, data),
      editRule: (data: EditAclRuleRequest) => client.put(`/acl/rule/${data.id}`, data),
      applyRules: (rules: number[]) =>
        client.put(`/acl/rule/apply`, {
          rules,
        }),
      deleteRule: (ruleId: number | string) => client.delete(`/acl/rule/${ruleId}`),
    },
  },
  activityLogStream: {
    getStreams: () => client.get<ActivityLogStream[]>('/activity_log_stream'),
    createStream: (data: CreateActivityLogStreamRequest) =>
      client.post('/activity_log_stream', data),
    updateStream: (id: number, data: CreateActivityLogStreamRequest) =>
      client.put(`/activity_log_stream/${id}`, data),
    deleteStream: (id: number) => client.delete(`/activity_log_stream/${id}`),
  },
  getActivityLog: (data?: ActivityLogRequestParams) =>
    client
      .get<PaginatedResponse<ActivityLogEvent>>(`/activity_log`, {
        params: data,
      })
      .then((resp) => resp.data),
  info: () => client.get<ApplicationInfo>('/info'),
  getLicenseInfo: () => client.get<LicenseInfoResponse>(`/enterprise_info`),
} as const;

export default api;
