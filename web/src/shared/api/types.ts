import type {
  ActivityLogEventTypeValue,
  ActivityLogModuleValue,
} from './activity-log-types';

export type Resource = object & { id: number };

export type ResourceById<T extends object> = {
  [id: number]: T | undefined;
};

export interface GatewayTokenResponse {
  grpc_url: string;
  token: string;
}

export interface CreateCARequest {
  common_name: string;
  email: string;
  validity_period_years: number;
}

export interface GetCAResponse {
  ca_cert_pem: string;
  subject_common_name: string;
  not_before: string;
  not_after: string;
  valid_for_days: number;
}

export interface UploadCARequest {
  cert_file: string;
}

export interface CreateAdminRequest {
  first_name: string;
  last_name: string;
  username: string;
  email: string;
  password: string;
}

export interface SetGeneralConfigRequest {
  defguard_url: string;
  default_admin_group_name: string;
  default_authentication: number;
  default_mfa_code_lifetime: number;
  public_proxy_url: string;
  admin_username: string;
}

export interface ValidateDeviceIpsRequest {
  ips: string[];
  locationId: number;
}

export interface IpValidation {
  valid: boolean;
  available: boolean;
}
export interface AvailableLocationIP {
  ip: string;
  network_part: string;
  modifiable_part: string;
  network_prefix: string;
}

export type AvailableLocationIpResponse = AvailableLocationIP[];

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
  name: string;
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
  phone: string | null;
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
  message?: string;
}

export interface AppInfoExceededLimits {
  user: boolean;
  device: boolean;
  wireguard_network: boolean;
}

export interface LicenseAppInfo {
  enterprise: boolean;
  limits_exceeded: AppInfoExceededLimits;
  any_limit_exceeded: boolean;
  is_enterprise_free: boolean;
  tier?: string | null;
}

export interface LimitInfo {
  current: number;
  limit: number;
}

export interface LicenseLimitsInfo {
  locations: LimitInfo;
  users: LimitInfo;
  user_devices: LimitInfo | null;
  network_devices: LimitInfo | null;
  devices: LimitInfo | null;
}

export const LicenseTier = {
  Business: 'Business',
  Enterprise: 'Enterprise',
} as const;

export type LicenseTierValue = (typeof LicenseTier)[keyof typeof LicenseTier];

export interface LicenseInfo {
  free: boolean;
  expired: boolean;
  limits_exceeded: boolean;
  subscription: boolean;
  valid_until: string | null;
  tier: LicenseTierValue;
  limits: LicenseLimitsInfo | null;
}

export interface LicenseInfoResponse {
  license_info: LicenseInfo | null;
}

export interface LdapInfo {
  enabled: boolean;
  ad: boolean;
}

export interface ApplicationInfo {
  version: string;
  network_present: boolean;
  smtp_enabled: boolean;
  external_openid_enabled: boolean;
  license_info: LicenseAppInfo;
  ldap_info: LdapInfo;
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

export interface Webhook {
  id: number;
  url: string;
  description: string;
  token: string;
  enabled: boolean;
  on_user_created: boolean;
  on_user_deleted: boolean;
  on_user_modified: boolean;
  on_hwkey_provision: boolean;
}

export type AddWebhookRequest = Omit<Webhook, 'id'>;

export interface ChangeWebhookStateRequest {
  id: number;
  enabled: boolean;
}

export interface NetworkDevice {
  id: number;
  name: string;
  assigned_ips: string[];
  description?: string;
  added_by: string;
  added_date: string;
  configured: boolean;
  // when configured is false this will be empty
  wireguard_pubkey?: string;
  location: {
    id: number;
    name: string;
  };
  split_ips: [
    {
      network_part: string;
      modifiable_part: string;
      network_prefix: string;
    },
  ];
}

export const LocationMfaMode = {
  Disabled: 'disabled',
  Internal: 'internal',
  External: 'external',
} as const;

export type LocationMfaModeValue = (typeof LocationMfaMode)[keyof typeof LocationMfaMode];

export type DeviceConfigResponse = {
  address: string;
  allowed_ips: string[];
  config: string;
  endpoint: string;
  keepalive_interval: number;
  network_id: number;
  network_name: string;
  pubkey: string;
  location_mfa_mode: LocationMfaModeValue;
};

export type AddNetworkDeviceResponse = {
  config: DeviceConfigResponse;
  device: NetworkDevice;
};

export type EditNetworkDeviceRequest = {
  id: number;
  assigned_ips: string[];
  description?: string;
  name: string;
};

export interface AddNetworkDeviceRequest {
  name: string;
  location_id: number;
  assigned_ips: string[];
  wireguard_pubkey?: string | null;
  description?: string | null;
}

export interface NetworkLocation {
  id: number;
  name: string;
  address: string[];
  port: number;
  endpoint: string;
  connected: boolean;
  connected_at: string | null;
  gateways: GatewayStatus[];
  allowed_ips: string[];
  allowed_groups: string[];
  dns: string | null;
  keepalive_interval: number;
  mtu: number;
  fwmark: number;
  peer_disconnect_threshold: number;
  acl_enabled: boolean;
  acl_default_allow: boolean;
  location_mfa_mode: LocationMfaModeValue;
  service_location_mode: LocationServiceModeValue;
}

export interface EditNetworkLocation
  extends Omit<
    NetworkLocation,
    'gateways' | 'connected_at' | 'id' | 'connected' | 'allowed_ips' | 'address'
  > {
  allowed_ips: string;
  address: string;
}

export interface EditNetworkLocationRequest {
  id: number;
  data: EditNetworkLocation;
}

export interface GatewayStatus {
  connected: boolean;
  network_id: number;
  network_name: string;
  name?: string;
  hostname: string;
  uid: string;
}
export interface TransferStats {
  collected_at: string;
  download: number;
  upload: number;
}

export interface LocationStats {
  active_users: number;
  active_user_devices: number;
  active_network_devices: number;
  current_active_users: number;
  current_active_user_devices: number;
  current_active_network_devices: number;
  upload: number;
  download: number;
  transfer_series: TransferStats[];
}

export interface LocationStatsRequest {
  id: number;
  // filter param
  from?: number;
}

export interface DeleteGatewayRequest {
  networkId: number | string;
  gatewayId: number | string;
}

export interface DeviceStats {
  connected_at: string;
  id: number;
  name: string;
  public_ip: string;
  wireguard_ips: string[];
  stats: TransferStats[];
}

export interface LocationUserDeviceStats {
  user: User;
  devices: DeviceStats[];
}

export interface LocationDevicesStats {
  user_devices: LocationUserDeviceStats[];
  network_devices: DeviceStats[];
}

export const LocationServiceMode = {
  Disabled: 'disabled',
  Prelogon: 'prelogon',
  Alwayson: 'alwayson',
} as const;

export type LocationServiceModeValue =
  (typeof LocationServiceMode)[keyof typeof LocationServiceMode];

export const ClientTrafficPolicy = {
  None: 'none',
  DisableAllTraffic: 'disable_all_traffic',
  ForceAllTraffic: 'force_all_traffic',
} as const;

export type ClientTrafficPolicyValue =
  (typeof ClientTrafficPolicy)[keyof typeof ClientTrafficPolicy];

export interface SettingsEnterprise {
  admin_device_management: boolean;
  client_traffic_policy: ClientTrafficPolicyValue;
  only_client_activation: boolean;
}

export type InitialSetupStepValue =
  | 'Welcome'
  | 'AdminUser'
  | 'GeneralConfiguration'
  | 'Ca'
  | 'CaSummary'
  | 'EdgeComponent'
  | 'Confirmation'
  | 'Finished';

export interface SettingsEssentials {
  initial_setup_completed: boolean;
  initial_setup_step: InitialSetupStepValue;
}

export const SmtpEncryption = {
  None: 'None',
  StartTls: 'StartTls',
  ImplicitTls: 'ImplicitTls',
} as const;

export type SmtpEncryptionValue = (typeof SmtpEncryption)[keyof typeof SmtpEncryption];

export interface SettingsSMTP {
  smtp_encryption: SmtpEncryptionValue;
  smtp_server: string | null;
  smtp_port: number | null;
  smtp_user: string | null;
  smtp_password: string | null;
  smtp_sender: string | null;
}

export interface SettingsEnrollment {
  enrollment_vpn_step_optional: boolean;
  enrollment_welcome_message: string;
  enrollment_welcome_email: string;
  enrollment_welcome_email_subject: string;
  enrollment_use_welcome_message_as_email: boolean;
}
export interface SettingsModules {
  openid_enabled: boolean;
  wireguard_enabled: boolean;
  webhooks_enabled: boolean;
  worker_enabled: boolean;
}

export interface SettingsBranding {
  instance_name: string;
  main_logo_url: string;
  nav_logo_url: string;
}

export interface SettingsLDAP {
  ldap_bind_password?: string;
  ldap_bind_username?: string;
  ldap_url?: string;
  ldap_group_member_attr: string;
  ldap_group_obj_class: string;
  ldap_group_search_base: string;
  ldap_groupname_attr: string;
  ldap_member_attr: string;
  ldap_user_obj_class: string;
  ldap_user_auxiliary_obj_classes: string[];
  ldap_user_search_base: string;
  ldap_username_attr: string;
  ldap_enabled: boolean;
  ldap_sync_enabled: boolean;
  ldap_is_authoritative: boolean;
  ldap_use_starttls: boolean;
  ldap_tls_verify_cert: boolean;
  ldap_sync_interval: number;
  ldap_uses_ad: boolean;
  ldap_user_rdn_attr?: string;
  ldap_sync_groups: string[];
}

export interface SettingsOpenID {
  openid_create_account: boolean;
}

export interface SettingsLicense {
  license: string | null;
}

export interface SettingsGatewayNotifications {
  gateway_disconnect_notifications_enabled: boolean;
  gateway_disconnect_notifications_inactivity_threshold: number;
  gateway_disconnect_notifications_reconnect_notification_enabled: boolean;
}

export type Settings = SettingsBranding &
  SettingsGatewayNotifications &
  SettingsEnterprise &
  SettingsLDAP &
  SettingsLicense &
  SettingsModules &
  SettingsOpenID &
  SettingsEnrollment &
  SettingsSMTP;

export interface OpenIdProviderSettings {
  create_account: boolean;
  username_handling: OpenIdProviderUsernameHandlingValue;
}

export const OpenIdProviderKind = {
  Custom: 'Custom',
  Google: 'Google',
  Microsoft: 'Microsoft',
  Okta: 'Okta',
  JumpCloud: 'JumpCloud',
  Zitadel: 'Zitadel',
} as const;

export type OpenIdProviderKindValue =
  (typeof OpenIdProviderKind)[keyof typeof OpenIdProviderKind];

export const DirectorySyncBehavior = {
  Keep: 'keep',
  Disable: 'disable',
  Delete: 'delete',
} as const;

export type DirectorySyncBehaviorValue =
  (typeof DirectorySyncBehavior)[keyof typeof DirectorySyncBehavior];

export const DirectorySyncTarget = {
  All: 'all',
  Users: 'users',
  Groups: 'groups',
} as const;

export type DirectorySyncTargetValue =
  (typeof DirectorySyncTarget)[keyof typeof DirectorySyncTarget];

export const OpenIdProviderUsernameHandling = {
  RemoveForbidden: 'RemoveForbidden',
  ReplaceForbidden: 'ReplaceForbidden',
  PruneEmailDomain: 'PruneEmailDomain',
} as const;

export type OpenIdProviderUsernameHandlingValue =
  (typeof OpenIdProviderUsernameHandling)[keyof typeof OpenIdProviderUsernameHandling];

export interface OpenIdProvider {
  id: number;
  name: OpenIdProviderKindValue;
  base_url: string;
  kind: OpenIdProviderKindValue;
  client_id: string;
  client_secret: string;
  display_name: string;
  google_service_account_key?: string | null;
  google_service_account_email?: string | null;
  admin_email?: string | null;
  directory_sync_enabled: boolean;
  directory_sync_interval: number;
  directory_sync_user_behavior: DirectorySyncBehaviorValue;
  directory_sync_admin_behavior: DirectorySyncBehaviorValue;
  directory_sync_target: DirectorySyncTargetValue;
  okta_private_jwk?: string | null;
  okta_dirsync_client_id?: string | null;
  directory_sync_group_match?: string | null;
  jumpcloud_api_key?: string | null;
  prefetch_users: boolean;
}

export interface OpenIdProviders {
  settings: OpenIdProviderSettings;
  provider: OpenIdProvider | null;
}

export type OpenIdProvidersResponse = OpenIdProviders | undefined;

export type AddOpenIdProvider = Omit<OpenIdProvider, 'id'> & OpenIdProviderSettings;

export interface TestDirectorySyncResponse {
  success: boolean;
  message: string | null;
}

export const AclStatus = {
  New: 'New',
  Applied: 'Applied',
  Modified: 'Modified',
  Deleted: 'Deleted',
  Expired: 'Expired',
} as const;

export type AclStatusValue = (typeof AclStatus)[keyof typeof AclStatus];

export const AclAliasStatus = {
  Applied: AclStatus.Applied,
  Modified: AclStatus.Modified,
} as const;

export type AclAliasStatusValue = (typeof AclAliasStatus)[keyof typeof AclAliasStatus];

export const AclDeploymentState = {
  Applied: AclStatus.Applied,
  Modified: AclStatus.Modified,
} as const;

export type AclDeploymentStateValue =
  (typeof AclDeploymentState)[keyof typeof AclDeploymentState];

export const AclProtocol = {
  TCP: 6,
  UDP: 17,
  ICMP: 1,
} as const;

export const aclProtocolValues = Object.values(AclProtocol);

export type AclProtocolValue = (typeof AclProtocol)[keyof typeof AclProtocol];

export const AclProtocolName: Record<AclProtocolValue, string> = {
  '1': 'ICMP',
  '6': 'TCP',
  '17': 'UDP',
};

export interface AclDestination {
  id: number;
  name: string;
  state: AclDeploymentStateValue;
  addresses: string;
  ports: string;
  protocols: AclProtocolValue[];
  rules: number[];
  any_address: boolean;
  any_port: boolean;
  any_protocol: boolean;
}

export type AddAclDestination = Omit<AclDestination, 'id' | 'state' | 'rules'>;

export type EditAclDestination = Omit<AclDestination, 'state' | 'rules'>;

export interface AclAlias {
  id: number;
  name: string;
  state: AclDeploymentStateValue;
  addresses: string;
  ports: string;
  protocols: AclProtocolValue[];
  rules: number[];
}

export type AddAclAliasRequest = Omit<AclAlias, 'id' | 'state' | 'rules'>;

export type EditAclAliasRequest = Omit<AclAlias, 'state' | 'rules'>;

export interface AclRule {
  id: number;
  state: AclStatusValue;
  name: string;
  all_locations: boolean;
  allow_all_users: boolean;
  deny_all_users: boolean;
  allow_all_groups: boolean;
  deny_all_groups: boolean;
  allow_all_network_devices: boolean;
  deny_all_network_devices: boolean;
  locations: number[];
  enabled: boolean;
  allowed_users: number[];
  denied_users: number[];
  allowed_groups: number[];
  denied_groups: number[];
  allowed_network_devices: number[];
  denied_network_devices: number[];
  addresses: string;
  ports: string;
  protocols: number[];
  expires: string | null;
  parent_id: number | null;
  any_address: boolean;
  any_port: boolean;
  any_protocol: boolean;
  use_manual_destination_settings: boolean;
  aliases: number[];
  destinations: number[];
}

export type EditAclRuleRequest = Omit<AclRule, 'state' | 'parent_id'>;

export type AddAclRuleRequest = Omit<AclRule, 'state' | 'parent_id' | 'id'>;

export interface OpenIdAuthInfo {
  url: string;
  button_display_name?: string | null;
}

export interface ActivityLogEvent {
  id: number;
  timestamp: string;
  user_id: number;
  username: string;
  location?: string;
  ip: string;
  event: ActivityLogEventTypeValue;
  module: ActivityLogModuleValue;
  device: string;
  description?: string;
}

export const ActivityLogStreamType = {
  VectorHttp: 'vector_http',
  LogstashHttp: 'logstash_http',
} as const;

export type ActivityLogStreamTypeValue =
  (typeof ActivityLogStreamType)[keyof typeof ActivityLogStreamType];

export interface ActivityLogStream {
  id: number;
  name: string;
  stream_type: ActivityLogStreamTypeValue;
  config: ActivityLogStreamConfig;
}
export interface CreateActivityLogStreamRequest {
  name: string;
  stream_type: ActivityLogStreamTypeValue;
  stream_config: ActivityLogStreamConfig;
}

export interface ActivityLogStreamConfig {
  url: string;
  username: string | null;
  password: string | null;
  cert: string | null;
}

export type ActivityLogSortKey =
  | 'timestamp'
  | 'username'
  | 'location'
  | 'ip'
  | 'event'
  | 'module'
  | 'device';

export interface Edge {
  id: number;
  name: string;
  address: string | null;
  port: number | null;
  version: string | null;
  connected_at: string | null;
  disconnected_at: string | null;
  modified_at: string;
  modified_by: number;
}

export interface EdgeInfo extends Edge {
  modified_by_firstname: string;
  modified_by_lastname: string;
}

export interface PaginationParams {
  page?: number;
}

export interface PaginationMeta {
  current_page: number;
  page_size: number;
  total_items: number;
  total_pagers: number;
  next_page: number | null;
}

export type PaginatedResponse<T> = {
  data: T[];
  pagination: PaginationMeta;
};

export type RequestSortParams<T> = {
  sort_by?: keyof T;
  sort_order?: SortDirectionValue;
};

export const SortDirection = {
  ASC: 'asc',
  DESC: 'desc',
} as const;

export type SortDirectionValue = (typeof SortDirection)[keyof typeof SortDirection];

export interface ActivityLogFilters {
  from: string;
  until: string;
  username: string[];
  location: string[];
  event: ActivityLogEventTypeValue[];
  module: ActivityLogModuleValue[];
  search: string;
}

export type ActivityLogRequestParams = Partial<ActivityLogFilters> &
  RequestSortParams<ActivityLogSortKey> &
  PaginationParams;
