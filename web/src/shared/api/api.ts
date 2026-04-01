import dayjs from 'dayjs';
import { cloneDeep } from 'lodash-es';
import { removeEmptyStrings } from '../utils/removeEmptyStrings';
import { client } from './api-client';
import { fetchAllPages, fetchPage } from './pagination';
import type {
  AclAlias,
  AclCount,
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
  AssignStaticIpsRequest,
  AuthKey,
  AvailableLocationIpResponse,
  ChangeAccountActiveRequest,
  ChangeWebhookStateRequest,
  CoreSelfSignedCertRequest,
  CountResponse,
  CreateActivityLogStreamRequest,
  CreateAdminRequest,
  CreateCARequest,
  CreateGroupRequest,
  DeleteApiTokenRequest,
  DeleteAuthKeyRequest,
  Device,
  DeviceLocationIpsResponse,
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
  Gateway,
  GatewayInfo,
  GetCAResponse,
  GetExternalSslInfoResponse,
  GetInternalSslInfoResponse,
  GroupInfo,
  IpValidation,
  LicenseCheckResponse,
  LicenseInfoResponse,
  LocationConnectedNetworkDevice,
  LocationConnectedNetworkDevicesRequest,
  LocationConnectedUser,
  LocationConnectedUserDevice,
  LocationConnectedUserDevicesRequest,
  LocationConnectedUsersRequest,
  LocationDevicesResponse,
  LocationStats,
  LocationStatsRequest,
  LoginRequest,
  LoginResponse,
  LoginResponseBasic,
  MfaCompleteResponse,
  MigrationWizardApiState,
  NetworkDevice,
  NetworkLocation,
  OpenIdAuthInfo,
  OpenIdClient,
  OpenIdProvidersResponse,
  RenameApiTokenRequest,
  RenameAuthKeyRequest,
  ResourceDisplay,
  SessionInfo,
  SetAutoAdoptionExternalUrlSettingsRequest,
  SetAutoAdoptionExternalUrlSettingsResponse,
  SetAutoAdoptionInternalUrlSettingsRequest,
  SetAutoAdoptionInternalUrlSettingsResponse,
  SetAutoAdoptionMfaSettingsRequest,
  SetAutoAdoptionVpnSettingsRequest,
  SetGeneralConfigRequest,
  Settings,
  SettingsEnterprise,
  SettingsEssentials,
  SetupAutoAdoptionResponse,
  StartEnrollmentRequest,
  StartEnrollmentResponse,
  TestDirectorySyncResponse,
  TotpInitResponse,
  UpdateInfo,
  UploadCARequest,
  User,
  UserChangePasswordRequest,
  UserDevice,
  UserProfileResponse,
  ValidateDeviceIpsRequest,
  ValidateIpAssignmentRequest,
  WebauthnLoginStartResponse,
  WebauthnRegisterFinishRequest,
  WebauthnRegisterStartResponse,
  Webhook,
  WizardState,
} from './types';

const api = {
  initial_setup: {
    createCA: (data: CreateCARequest) => client.post('/initial_setup/ca', data),
    getCA: () => client.get<GetCAResponse>('/initial_setup/ca'),
    uploadCA: (data: UploadCARequest) => client.post('/initial_setup/ca/upload', data),
    createAdminUser: (data: CreateAdminRequest) =>
      client.post('/initial_setup/admin', data),
    login: (data: LoginRequest) => client.post('/initial_setup/login', data),
    session: () => client.get('/initial_setup/session'),
    getAutoAdoptionResult: () =>
      client.get<SetupAutoAdoptionResponse>('/initial_setup/auto_adoption'),
    getWizardState: () => client.get<WizardState>('/wizard'),
    setGeneralConfig: (data: SetGeneralConfigRequest) =>
      client.post('/initial_setup/general_config', data),
    setAutoAdoptionInternalUrlSettings: (
      data: SetAutoAdoptionInternalUrlSettingsRequest,
    ) =>
      client.post<SetAutoAdoptionInternalUrlSettingsResponse>(
        '/initial_setup/auto_wizard/internal_url_settings',
        data,
      ),
    getInternalSslInfo: () =>
      client.get<GetInternalSslInfoResponse>(
        '/initial_setup/auto_wizard/internal_url_settings',
      ),
    setAutoAdoptionExternalUrlSettings: (
      data: SetAutoAdoptionExternalUrlSettingsRequest,
    ) =>
      client.post<SetAutoAdoptionExternalUrlSettingsResponse>(
        '/initial_setup/auto_wizard/external_url_settings',
        data,
      ),
    getExternalSslInfo: () =>
      client.get<GetExternalSslInfoResponse>(
        '/initial_setup/auto_wizard/external_url_settings',
      ),
    setAutoAdoptionVpnSettings: (data: SetAutoAdoptionVpnSettingsRequest) =>
      client.post('/initial_setup/auto_wizard/vpn_settings', data),
    setAutoAdoptionMfaSettings: (data: SetAutoAdoptionMfaSettingsRequest) =>
      client.post('/initial_setup/auto_wizard/mfa_settings', data),
    finishSetup: () => client.post('/initial_setup/finish'),
  },
  openid: {
    authInfo: () => client.get<OpenIdAuthInfo>(`/openid/auth_info`),
    callback: (data: unknown) => client.post<LoginResponse>(`/openid/callback`, data),
  },
  openIdClient: {
    getOpenIdClients: () => fetchAllPages<OpenIdClient>(`/oauth`),
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
    getGroups: () => fetchAllPages<string>('/group'),
    getGroupsInfo: () => client.get<GroupInfo[]>('/group-info'),
    editGroup: ({ id, ...data }: EditGroupRequest) => client.put(`/group/${id}`, data),
    deleteGroup: (id: number) => client.delete(`/group/${id}`),
    addUsersToGroups: (data: AddUsersToGroupsRequest) =>
      client.post(`/groups-assign`, data),
  },
  app: {
    info: () => client.get<ApplicationInfo>('/info'),
    updates: () => client.get<UpdateInfo | null>('/updates'),
  },
  user: {
    addUser: (data: AddUserRequest) => client.post<User>('/user', data),
    usernameAvailable: (username: string) =>
      client.post('/user/available', {
        username,
      }),
    getMe: () => client.get<User>('/me'),
    getUsers: () => fetchAllPages<User>('/user'),
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
    getDevices: () => fetchAllPages<NetworkDevice>('/device/network'),
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
        device_id: data.deviceId,
      }),
  },
  location: {
    getCount: () => client.get<CountResponse>(`/network/count`),
    getLocationsDisplay: () => client.get<ResourceDisplay[]>(`/network/display`),
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
      client.get<GatewayInfo[]>(`/network/${id}/gateways`),
    getLocationConnectedUsers: ({ id, ...params }: LocationConnectedUsersRequest) =>
      fetchPage<LocationConnectedUser>(`/network/${id}/stats/connected_users`, {
        ...params,
        from: params.from
          ? dayjs.utc().subtract(params.from, 'hour').toISOString()
          : undefined,
      }),
    getLocationConnectedNetworkDevices: ({
      id,
      ...params
    }: LocationConnectedNetworkDevicesRequest) =>
      fetchPage<LocationConnectedNetworkDevice>(
        `/network/${id}/stats/connected_network_devices`,
        {
          ...params,
          from: params.from
            ? dayjs.utc().subtract(params.from, 'hour').toISOString()
            : undefined,
        },
      ),
    getLocationConnectedUserDevices: ({
      locationId,
      userId,
      from,
    }: LocationConnectedUserDevicesRequest) =>
      client
        .get<LocationConnectedUserDevice[]>(
          `/network/${locationId}/stats/connected_users/${userId}/devices`,
          {
            params: {
              from: from ? dayjs.utc().subtract(from, 'hour').toISOString() : undefined,
            },
          },
        )
        .then((resp) => resp.data),
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
    getDeviceConfigs: async (device: Device): Promise<AddDeviceResponse> => {
      const { data: configs } = await client.get<AddDeviceResponseConfig[]>(
        `/device/network/${device.id}/config`,
      );
      return {
        configs,
        device,
      };
    },
    getUserDeviceIps: (username: string) =>
      client.get<LocationDevicesResponse>(`/device/user/${username}/ip`),
    getDeviceIps: (username: string, deviceId: number) =>
      client.get<DeviceLocationIpsResponse>(`/device/user/${username}/ip/${deviceId}`),
    assignUserDeviceIps: (username: string, data: AssignStaticIpsRequest) =>
      client.post(`/device/user/${username}/ip`, data),
    validateUserDeviceIp: (username: string, data: ValidateIpAssignmentRequest) =>
      client.post(`/device/user/${username}/ip/validate`, data),
  },
  settings: {
    getSettings: () => client.get<Settings>('/settings'),
    editSettings: (data: Settings) => client.put('/settings', data),
    patchSettings: (data: Partial<Settings>) => client.patch('/settings', data),
    getEnterpriseSettings: () => client.get<SettingsEnterprise>('/settings_enterprise'),
    patchEnterpriseSettings: (data: Partial<SettingsEnterprise>) =>
      client.patch('/settings_enterprise', data),
    getSettingsEssentials: () => client.get<SettingsEssentials>('/settings_essentials'),
    getLdapConnectionStatus: () => client.get(`/ldap/test`),
  },
  openIdProvider: {
    getOpenIdProvider: () =>
      client.get<OpenIdProvidersResponse>('/openid/provider/current'),
    addOpenIdProvider: (data: AddOpenIdProvider) => client.post('/openid/provider', data),
    deleteOpenIdProvider: (name: string) => client.delete(`/openid/provider/${name}`),
    editOpenIdProvider: (data: AddOpenIdProvider) =>
      client.put(`/openid/provider/${data.name}`, data),
    testDirectorySync: () =>
      client.get<TestDirectorySyncResponse>(`/test_directory_sync`),
  },
  mail: {
    sendTestEmail: (data: { to: string }) => client.post('/mail/test', data),
  },
  edge: {
    getEdges: () => client.get<EdgeInfo[]>('/proxy'),
    getEdge: (edgeId: number | string) => client.get<Edge>(`/proxy/${edgeId}`),
    editEdge: (data: Edge) => client.put(`/proxy/${data.id}`, data),
    deleteEdge: (edgeId: number | string) => client.delete(`/proxy/${edgeId}`),
  },
  gateway: {
    getGateways: () => client.get<GatewayInfo[]>('/gateway'),
    getGateway: (gatewayId: number | string) =>
      client.get<Gateway>(`/gateway/${gatewayId}`),
    editGateway: (data: { id: number | string; name: string; enabled: boolean }) =>
      client.put(`/gateway/${data.id}`, {
        name: data.name,
        enabled: data.enabled,
      }),
    deleteGateway: (gatewayId: number | string) => client.delete(`/gateway/${gatewayId}`),
  },
  core: {
    certSelfSigned: (data: CoreSelfSignedCertRequest) =>
      client.post('/core/cert/self-signed', data),
    certUpload: (data: { cert_pem: string; key_pem: string }) =>
      client.post('/core/cert/upload', data),
    getCA: () => client.get<GetCAResponse>('/core/cert/ca'),
  },
  acl: {
    destination: {
      getCount: () => client.get<AclCount>('acl/destination/count'),
      getDestinations: () => client.get<AclDestination[]>('/acl/destination'),
      getDestination: (destinationId: number | string) =>
        client.get<AclDestination>(`/acl/destination/${destinationId}`),
      addDestination: (data: AddAclDestination) => client.post(`/acl/destination`, data),
      editDestination: (data: EditAclDestination) =>
        client.put(`/acl/destination/${data.id}`, data),
      deleteDestination: (destinationId: number | string) =>
        client.delete(`/acl/destination/${destinationId}`),
      applyDestinations: (destinations: number[]) =>
        client.put(`/acl/destination/apply`, {
          destinations,
        }),
    },
    alias: {
      getCount: () => client.get<AclCount>('acl/alias/count'),
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
      getCount: () => client.get<AclCount>('acl/rule/count'),
      getRules: () => fetchAllPages<AclRule>(`/acl/rule`),
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
  migration: {
    finish: () => client.post(`/migration/finish`),
    ca: {
      createCA: (data: CreateCARequest) => client.post('/migration/ca', data),
      getCA: () => client.get<GetCAResponse>('/migration/ca'),
    },
    state: {
      getMigrationState: () =>
        client.get<MigrationWizardApiState | null>(`/migration/state`),
      updateMigrationState: (data: MigrationWizardApiState) =>
        client.put(`/migration/state`, data),
    },
    setInternalUrlSettings: (data: SetAutoAdoptionInternalUrlSettingsRequest) =>
      client.post<SetAutoAdoptionInternalUrlSettingsResponse>(
        '/migration/internal_url_settings',
        data,
      ),
    getInternalSslInfo: () =>
      client.get<GetInternalSslInfoResponse>('/migration/internal_url_settings'),
    setExternalUrlSettings: (data: SetAutoAdoptionExternalUrlSettingsRequest) =>
      client.post<SetAutoAdoptionExternalUrlSettingsResponse>(
        '/migration/external_url_settings',
        data,
      ),
    getExternalSslInfo: () =>
      client.get<GetExternalSslInfoResponse>('/migration/external_url_settings'),
  },
  checkLicense: (data: { license: string }) =>
    client.post<LicenseCheckResponse>('/license/check', data),
  getSessionInfo: () => client.get<SessionInfo>(`/session-info`),
  getActivityLog: (data?: ActivityLogRequestParams) =>
    fetchPage<ActivityLogEvent>(`/activity_log`, data),
  info: () => client.get<ApplicationInfo>('/info'),
  getLicenseInfo: () => client.get<LicenseInfoResponse>(`/enterprise_info`),
  support: {
    getSupportData: () => client.get<object>('/support/configuration'),
    getLogs: () => client.get<string>('/support/logs'),
  },
} as const;

export default api;
