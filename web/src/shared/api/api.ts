import { client } from './api-client';
import type {
  AddApiTokenRequest,
  AddApiTokenResponse,
  AddAuthKeyRequest,
  AddDeviceRequest,
  AddDeviceResponse,
  AddDeviceResponseConfig,
  AdminChangeUserPasswordRequest,
  ApiToken,
  ApplicationInfo,
  AuthKey,
  DeleteApiTokenRequest,
  DeleteAuthKeyRequest,
  Device,
  EnableMfaMethodResponse,
  LoginRequest,
  LoginResponse,
  LoginResponseBasic,
  MfaCompleteResponse,
  RenameApiTokenRequest,
  RenameAuthKeyRequest,
  StartEnrollmentRequest,
  StartEnrollmentResponse,
  TotpInitResponse,
  User,
  UserChangePasswordRequest,
  UserDevice,
  UserProfileResponse,
  WebauthnLoginStartResponse,
  WebauthnRegisterFinishRequest,
  WebauthnRegisterStartResponse,
} from './types';

const api = {
  app: {
    info: () => client.get<ApplicationInfo>('/info'),
  },
  user: {
    getMe: client.get<User>('/me'),
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
