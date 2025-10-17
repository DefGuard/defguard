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
  totp_enabled: boolean;
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

export interface DeviceNetworkInfo {
  device_wireguard_ips: string[];
  is_active: boolean;
  network_gateway_ip: string;
  network_id: number;
  network_name: string;
  last_connected_at?: string;
  last_connected_ip?: string;
}

export interface Device {
  id: number;
  user_id: number;
  name: string;
  wireguard_pubkey: string;
  created: string;
  networks: DeviceNetworkInfo[];
}

export interface SecurityKey {
  id: number;
  name: string;
}

export type UserProfileResponse = {
  user: User;
  devices: Device[];
  security_keys: SecurityKey[];
  biometric_enabled_devices: number[];
};

export interface UserDevice extends Device {
  biometry_enabled: boolean;
}

export interface UserProfile {
  user: User;
  devices: UserDevice[];
  security_keys: SecurityKey[];
}
