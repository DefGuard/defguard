export type AddUsersToGroupsRequest = {
  groups: string[];
  users: number[];
};
export interface GroupInfo {
  id: number;
  name: string;
  members: string[];
  vpn_locations: string[];
  is_admin: boolean;
}

export interface GroupsResponse {
  groups: string[];
}
export interface UsersListItem extends User {
  devices: Device[];
}

export interface EditGroupRequest extends CreateGroupRequest {
  originalName?: string;
}

export interface CreateGroupRequest {
  name: string;
  members?: string[];
  is_admin: boolean;
}

export const UserMfaMethod = {
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

export interface MfaCompleteResponse {
  user: User;
  url?: string;
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

export interface UserChangePasswordRequest {
  new_password: string;
  old_password: string;
}

export interface AdminChangeUserPasswordRequest {
  new_password: string;
  username: string;
}

export interface TotpInitResponse {
  secret: string;
}

export interface EnableMfaMethodResponse {
  codes?: string[];
}

export interface MfaFinishResponse {
  url?: string;
  user?: User;
}

export interface ApiError {
  msg?: string;
}

export interface LicenseLimits {
  user: boolean;
  device: boolean;
  wireguard_network: boolean;
}

export interface LicenseInfo {
  enterprise: boolean;
  limits_exceeded: LicenseLimits;
  any_limit_exceeded: boolean;
  is_enterprise_free: boolean;
}
export interface LdapInfo {
  enabled: boolean;
  ad: boolean;
}

export interface ApplicationInfo {
  version: string;
  network_present: boolean;
  smtp_enabled: boolean;
  license_info: LicenseInfo;
  ldap_info: LdapInfo;
  external_openid_enabled: boolean;
}

export interface WebauthnRegisterStartResponse {
  publicKey: PublicKeyCredentialCreationOptionsJSON;
}

export interface WebauthnRegisterFinishRequest {
  name: string;
  rpkc: PublicKeyCredentialJSON;
}

export interface WebauthnLoginStartResponse {
  publicKey: PublicKeyCredentialJSON;
}

export interface StartEnrollmentRequest {
  username: string;
  send_enrollment_notification: boolean;
  email?: string;
}

export interface StartEnrollmentResponse {
  enrollment_url: string;
  enrollment_token: string;
}

export interface AddDeviceRequest {
  username: string;
  name: string;
  wireguard_pubkey: string;
}

export interface AddDeviceResponseConfig {
  network_id: number;
  network_name: string;
  config: string;
}

export interface AddDeviceResponse {
  device: Omit<Device, 'networks'>;
  configs: AddDeviceResponseConfig[];
}

export const AuthKeyType = {
  SSH: 'ssh',
  GPG: 'gpg',
} as const;

export type AuthKeyTypeValue = (typeof AuthKeyType)[keyof typeof AuthKeyType];

export interface AddAuthKeyRequest {
  username: string;
  name: string;
  key: string;
  key_type: string;
}

export interface RenameAuthKeyRequest {
  id: number;
  username: string;
  name: string;
}

export interface DeleteAuthKeyRequest {
  username: string;
  id: number;
}

export interface AuthKey {
  id: number;
  user_id: number;
  key: string;
  key_type: AuthKeyTypeValue;
  // name is null when key was made by provisioning an yubikey
  name: string | null;
  yubikey_id: number | null;
  yubikey_name: string | null;
  yubikey_serial: string | null;
}

export interface AddApiTokenRequest {
  username: string;
  name: string;
}

export interface AddApiTokenResponse {
  token: string;
}

export interface ApiToken {
  id: number;
  name: string;
  created_at: string;
}

export interface RenameApiTokenRequest {
  id: number;
  name: string;
  username: string;
}

export interface DeleteApiTokenRequest {
  id: number;
  username: string;
}

export interface AddUserRequest {
  username: string;
  email: string;
  last_name: string;
  first_name: string;
  password?: string;
  phone?: string;
}

export interface ChangeAccountActiveRequest {
  username: string;
  active: boolean;
}

export interface OpenIdClient {
  id: string;
  name: string;
  client_id: string;
  client_secret: string;
  redirect_uri: string[];
  scope: OpenIdClientScopeValue[];
  enabled: boolean;
}

export type AddOpenIdClient = Omit<OpenIdClient, 'id' | 'client_id' | 'client_secret'>;

export interface EditOpenIdClientActiveStateRequest {
  client_id: string;
  enabled: boolean;
}

export const OpenIdClientScope = {
  OpenId: 'openid',
  Groups: 'groups',
  Email: 'email',
  Profile: 'profile',
  Phone: 'phone',
} as const;

export type OpenIdClientScopeValue =
  (typeof OpenIdClientScope)[keyof typeof OpenIdClientScope];
