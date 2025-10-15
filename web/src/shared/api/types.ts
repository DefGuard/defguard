const UserMfaMethod = {
  None: 'none',
  OneTimePassword: 'OneTimePassword',
  Email: 'Email',
  Webauthn: 'Webauthn',
} as const;

export type UserMfaMethodValue = (typeof UserMfaMethod)[keyof typeof UserMfaMethod];

export interface OAuth2AuthorizedApps {
  oauth2client_id: number;
  oauth2client_name: string;
  user_id: number;
}

export interface User {
  id: number;
  username: string;
  first_name: string;
  last_name: string;
  mfa_method: UserMfaMethodValue;
  mfa_enabled: boolean;
  email_mfa_enabled: boolean;
  email: string;
  groups: string[];
  is_active: boolean;
  enrolled: boolean;
  is_admin: boolean;
  ldap_pass_requires_change: boolean;
  phone?: string;
  authorized_apps?: OAuth2AuthorizedApps[];
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginMfaResponse {
  mfa_method: UserMfaMethodValue;
  totp_available: boolean;
  webauthn_available: boolean;
  email_available: boolean;
}

export interface LoginResponseBasic {
  url?: string;
  user?: User;
}

export type LoginResponse = LoginResponseBasic | LoginMfaResponse;
