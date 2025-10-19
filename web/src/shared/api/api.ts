import { client } from './api-client';
import type {
  AdminChangeUserPasswordRequest,
  LoginRequest,
  LoginResponse,
  User,
  UserChangePasswordRequest,
  UserProfileResponse,
} from './types';

const api = {
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
  },
  auth: {
    login: (data: LoginRequest) => client.post<LoginResponse>(`/auth`, data),
  },
  mfa: {
    enable: client.post('/auth/mfa'),
    disable: client.delete('/auth/mfa'),
    recovery: (code: string) => client.post('/auth/recovery', { code }),
    totp: {
      init: client.post('/auth/totp/init'),
      enable: (code: string) =>
        client.post('/auth/totp', {
          code,
        }),
      verify: (code: string) =>
        client.post('/auth/totp/verify', {
          code,
        }),
      disable: () => client.delete('/auth/totp'),
    },
    email: {
      init: client.post('/auth/email/init'),
      enable: (code: string) =>
        client.post('/auth/email', {
          code,
        }),
      disable: client.delete('/auth/delete'),
      resend: client.get('/auth/email'),
      verify: (code: string) => client.post('/auth/email/verify', { code }),
    },
    webauthn: {
      register: {
        start: (name: string) =>
          client.post('/auth/webauthn/init', {
            name,
          }),
        finish: (data: unknown) => client.post('/auth/webauthn/finish'),
      },
      login: {
        start: () => client.post('/auth/webauthn/start'),
        finish: () => client.post('/auth/webauthn'),
      },
    },
  },
} as const;

export default api;
