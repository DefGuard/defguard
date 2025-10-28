import { client } from './api-client';
import type {
  AdminChangeUserPasswordRequest,
  ApplicationInfo,
  EnableMfaMethodResponse,
  LoginRequest,
  LoginResponse,
  LoginResponseBasic,
  MfaCompleteResponse,
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
} as const;

export default api;
