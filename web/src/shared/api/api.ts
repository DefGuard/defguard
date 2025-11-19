import { cloneDeep } from 'lodash-es';
import { removeEmptyStrings } from '../utils/removeEmptyStrings';
import { client } from './api-client';
import type {
  AddApiTokenRequest,
  AddApiTokenResponse,
  AddAuthKeyRequest,
  AddDeviceRequest,
  AddDeviceResponse,
  AddDeviceResponseConfig,
  AddNetworkDeviceRequest,
  AddNetworkDeviceResponse,
  AddOpenIdClient,
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
  CreateGroupRequest,
  DeleteApiTokenRequest,
  DeleteAuthKeyRequest,
  Device,
  EditGroupRequest,
  EditNetworkDeviceRequest,
  EditOpenIdClientActiveStateRequest,
  EnableMfaMethodResponse,
  GroupInfo,
  GroupsResponse,
  IpValidation,
  LoginRequest,
  LoginResponse,
  LoginResponseBasic,
  MfaCompleteResponse,
  NetworkDevice,
  NetworkLocation,
  OpenIdClient,
  RenameApiTokenRequest,
  RenameAuthKeyRequest,
  StartEnrollmentRequest,
  StartEnrollmentResponse,
  TotpInitResponse,
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
    getMe: client.get<User>('/me'),
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
        disable: () => client.delete('/auth/totp'),
      },
      email: {
        init: () => client.post('/auth/email/init'),
        enable: (code: string) =>
          client.post<EnableMfaMethodResponse>('/auth/email', {
            code,
          }),
        disable: () => client.delete('/auth/delete'),
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
    getLocations: () => client.get<NetworkLocation[]>('/network'),
    getLocation: (id: number) => client.get<NetworkLocation>(`/network/${id}`),
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
} as const;

export default api;
