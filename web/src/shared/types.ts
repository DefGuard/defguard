import {
  CredentialCreationOptionsJSON,
  CredentialRequestOptionsJSON,
  PublicKeyCredentialWithAssertionJSON,
  PublicKeyCredentialWithAttestationJSON,
} from '@github/webauthn-json';
import { AxiosError, AxiosPromise } from 'axios';

import { UpdateInfo } from './hooks/store/useUpdatesStore';

export type ApiError = AxiosError<ApiErrorResponse>;

export type ApiErrorResponse = {
  msg?: string;
  message?: string;
};

export enum UserStatus {
  active = 'Active',
  inactive = 'Inactive',
  awaitingLogin = 'Awaiting login',
}

export enum UserMFAMethod {
  NONE = 'None',
  ONE_TIME_PASSWORD = 'OneTimePassword',
  EMAIL = 'Email',
  WEB_AUTH_N = 'Webauthn',
}

export enum AuthenticationKeyType {
  SSH = 'ssh',
  GPG = 'gpg',
}

export type User = {
  id: number;
  username: string;
  last_name: string;
  first_name: string;
  mfa_method: UserMFAMethod;
  mfa_enabled: boolean;
  totp_enabled: boolean;
  email_mfa_enabled: boolean;
  email: string;
  phone?: string;
  groups: string[];
  authorized_apps?: OAuth2AuthorizedApps[];
  is_active: boolean;
  enrolled: boolean;
  is_admin: boolean;
};

export type UserProfile = {
  user: User;
  devices: Device[];
  security_keys: SecurityKey[];
};

export interface OAuth2AuthorizedApps {
  oauth2client_id: string;
  oauth2client_name: string;
  user_id: string;
}

export interface SecurityKey {
  id: number;
  name: string;
}

export interface Location {
  name: string;
  ipAddress: string;
  shared: {
    ipAddress: string;
  }[];
}

export type AddDeviceResponseDevice = Omit<Device, 'networks'>;

export interface Device {
  id: number;
  user_id: number;
  name: string;
  wireguard_pubkey: string;
  created: string;
  networks: DeviceNetworkInfo[];
}

export type DeviceNetworkInfo = {
  device_wireguard_ip: string;
  is_active: boolean;
  network_gateway_ip: string;
  network_id: number;
  network_name: string;
  last_connected_at?: string;
  last_connected_ip?: string;
};

export interface AddDeviceRequest {
  username: string;
  name: string;
  wireguard_pubkey: string;
}

export type GatewayStatus = {
  connected: boolean;
  network_id: number;
  name?: string;
  hostname: string;
  uid: string;
};

export interface Network {
  id: number;
  name: string;
  address: string;
  port: number;
  endpoint: string;
  connected?: boolean;
  connected_at?: string;
  gateways?: GatewayStatus[];
  allowed_ips?: string[];
  allowed_groups?: string[];
  dns?: string;
  mfa_enabled: boolean;
  keepalive_interval: number;
  peer_disconnect_threshold: number;
}

export type ModifyNetworkRequest = {
  id: number;
  network: Omit<
    Network,
    'gateways' | 'connected' | 'id' | 'connected_at' | 'allowed_ips'
  > & {
    allowed_ips: string;
  };
};

export interface ImportNetworkRequest {
  name: string;
  endpoint: string;
  config: string;
  allowed_groups: string[];
}

export interface MapUserDevicesRequest {
  networkId: number;
  devices: MappedDevice[];
}

export interface NetworkToken {
  token: string;
  grpc_url: string;
}

export interface LoginData {
  username: string;
  password: string;
}

export interface CallbackData {
  code: string;
  state: string;
}

export type LoginSubjectData = {
  user?: User;
  // URL of an already authorized application
  url?: string;
  mfa?: MFALoginResponse;
};

export interface DeleteUserModal {
  visible: boolean;
  user?: User;
}

export interface ToggleUserModal {
  visible: boolean;
  user?: User;
}

export interface ProvisionKeyModal {
  visible: boolean;
  user?: User;
}

export interface AddAuthenticationKeyModal {
  visible: boolean;
  user?: User;
}

export interface DeleteAuthenticationKeyModal {
  visible: boolean;
  authenticationKey?: AuthenticationKey;
}

export interface AddApiTokenModal {
  visible: boolean;
  user?: User;
}

export interface DeleteApiTokenModal {
  visible: boolean;
  apiToken?: ApiToken;
}

export interface DeleteOpenidClientModal {
  visible: boolean;
  client?: OpenidClient;
  onSuccess?: () => void;
}

export interface EnableOpenidClientModal {
  visible: boolean;
  client?: OpenidClient;
  onSuccess?: () => void;
}

export interface GenericApiResponse {
  ok?: boolean;
}

export interface ChangePasswordRequest {
  new_password: string;
  username: string;
}

export interface ResetPasswordRequest {
  username: string;
}

export interface AddUserRequest {
  username: string;
  password?: string;
  email: string;
  last_name: string;
  first_name: string;
  phone?: string;
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
export interface GroupsResponse {
  groups: string[];
}

export interface UserGroupRequest {
  group: string;
  username: string;
}

export interface ChangeUserPasswordRequest {
  new_password: string;
  username: string;
}

export interface GetNetworkStatsRequest {
  /**UTC date parsed to ISO string. This sets how far back stats will be returned. */
  from?: string;
  id: Network['id'];
}

export interface UserEditRequest {
  username: string;
  data: Partial<User>;
}

export interface MFALoginResponse {
  mfa_method: UserMFAMethod;
  totp_available: boolean;
  webauthn_available: boolean;
  email_available: boolean;
}

export interface LoginResponse {
  url?: string;
  user?: User;
  mfa?: MFALoginResponse;
}

export interface OpenIdInfoResponse {
  url: string;
  button_display_name?: string;
}

export interface DeleteWebAuthNKeyRequest {
  username: User['username'];
  keyId: SecurityKey['id'];
}

export interface RecoveryCodes {
  codes: string[];
}

export interface RecoveryLoginRequest {
  code: string;
}

export type MFARecoveryCodesResponse = Promise<void | RecoveryCodes>;

export interface VersionResponse {
  version: string;
}

export interface MFAFinishResponse {
  url?: string;
  user?: User;
}

export interface ImportNetworkResponse {
  network: Network;
  devices: ImportedDevice[];
}

export interface ImportedDevice {
  name: string;
  wireguard_ip: string;
  wireguard_pubkey: string;
  user_id?: number;
}

export interface MappedDevice extends ImportedDevice {
  user_id: number;
}

export interface AppInfo {
  version: string;
  network_present: boolean;
  smtp_enabled: boolean;
  license_info: LicenseInfo;
}

export type GetDeviceConfigRequest = {
  device_id: number;
  network_id: number;
};

export type AddDeviceResponse = {
  device: AddDeviceResponseDevice;
  configs: AddDeviceConfig[];
};

export type DeleteGatewayRequest = {
  networkId: number;
  gatewayId: string;
};

export type ChangePasswordSelfRequest = {
  old_password: string;
  new_password: string;
};

export type AuthCodeRequest = {
  code: string;
};

export type AuthenticationKeyInfo = {
  id: number;
  name?: string;
  key_type: AuthenticationKeyType;
  key: string;
  yubikey_serial?: string;
  yubikey_id?: number;
  yubikey_name?: string;
};

export type AuthenticationKeyRequestBase = {
  username: string;
};

export type RenameAuthenticationKeyRequest = {
  id: number;
  name: string;
} & AuthenticationKeyRequestBase;

export type AddAuthenticationKeyRequest = {
  name: string;
  key: string;
  key_type: string;
} & AuthenticationKeyRequestBase;

export type ApiTokenInfo = {
  id: number;
  name?: string;
};

export type ApiTokenRequestBase = {
  username: string;
};

export type RenameApiTokenRequest = {
  id: number;
  name: string;
} & ApiTokenRequestBase;

export type AddApiTokenRequest = {
  name: string;
} & ApiTokenRequestBase;

export type AddApiTokenResponse = {
  token: string;
};

export type ModifyGroupsRequest = {
  name: string;
  // array of usernames
  members?: string[];
  is_admin: boolean;
};

export type AddUsersToGroupsRequest = {
  groups: string[];
  users: number[];
};

export type EditGroupRequest = ModifyGroupsRequest & {
  originalName: string;
};

export type AuthenticationKey = {
  id: number;
  name: string;
  key_type: AuthenticationKeyType;
  key: string;
};

export type ApiToken = {
  id: number;
  name: string;
  created_at: string;
};

export type EnterpriseInfoResponse = {
  license_info?: EnterpriseInfo;
};

export interface ApiHook {
  getAppInfo: () => Promise<AppInfo>;
  getNewVersion: () => Promise<UpdateInfo>;
  changePasswordSelf: (data: ChangePasswordSelfRequest) => Promise<EmptyApiResponse>;
  getEnterpriseInfo: () => Promise<EnterpriseInfoResponse>;
  oAuth: {
    consent: (params: unknown) => Promise<EmptyApiResponse>;
  };
  groups: {
    getGroupsInfo: () => Promise<GroupInfo[]>;
    getGroups: () => Promise<GroupsResponse>;
    createGroup: (data: ModifyGroupsRequest) => Promise<EmptyApiResponse>;
    editGroup: (data: EditGroupRequest) => Promise<EmptyApiResponse>;
    deleteGroup: (groupName: string) => Promise<EmptyApiResponse>;
    addUsersToGroups: (data: AddUsersToGroupsRequest) => Promise<EmptyApiResponse>;
  };
  user: {
    getMe: () => Promise<User>;
    addUser: (data: AddUserRequest) => Promise<User>;
    startEnrollment: (data: StartEnrollmentRequest) => Promise<StartEnrollmentResponse>;
    getUser: (username: string) => Promise<UserProfile>;
    getUsers: () => Promise<User[]>;
    editUser: (data: UserEditRequest) => Promise<User>;
    deleteUser: (user: User) => EmptyApiResponse;
    usernameAvailable: (username: string) => EmptyApiResponse;
    changePassword: (data: ChangePasswordRequest) => EmptyApiResponse;
    resetPassword: (data: ResetPasswordRequest) => EmptyApiResponse;
    addToGroup: (data: UserGroupRequest) => EmptyApiResponse;
    removeFromGroup: (data: UserGroupRequest) => EmptyApiResponse;
    startDesktopActivation: (
      data: StartEnrollmentRequest,
    ) => Promise<StartEnrollmentResponse>;
    getAuthenticationKeysInfo: (
      data: AuthenticationKeyRequestBase,
    ) => Promise<AuthenticationKeyInfo[]>;
    addAuthenticationKey: (data: AddAuthenticationKeyRequest) => EmptyApiResponse;
    deleteAuthenticationKey: (data: { id: number; username: string }) => EmptyApiResponse;
    renameAuthenticationKey: (data: {
      id: number;
      username: string;
      name: string;
    }) => EmptyApiResponse;
    renameYubikey: (data: {
      id: number;
      username: string;
      name: string;
    }) => EmptyApiResponse;
    deleteYubiKey: (data: { id: number; username: string }) => EmptyApiResponse;
    getApiTokensInfo: (data: ApiTokenRequestBase) => Promise<ApiToken[]>;
    addApiToken: (data: AddApiTokenRequest) => Promise<AddApiTokenResponse>;
    deleteApiToken: (data: { id: number; username: string }) => EmptyApiResponse;
    renameApiToken: (data: {
      id: number;
      username: string;
      name: string;
    }) => EmptyApiResponse;
  };
  standaloneDevice: {
    createManualDevice: (
      data: CreateStandaloneDeviceRequest,
    ) => Promise<CreateStandaloneDeviceResponse>;
    createCliDevice: (
      data: CreateStandaloneDeviceRequest,
    ) => Promise<StartEnrollmentResponse>;
    getDevice: (deviceId: number | string) => Promise<StandaloneDevice>;
    deleteDevice: (deviceId: number | string) => Promise<void>;
    editDevice: (data: StandaloneDeviceEditRequest) => Promise<void>;
    getAvailableIp: (
      data: GetAvailableLocationIpRequest,
    ) => Promise<GetAvailableLocationIpResponse>;
    validateLocationIp: (
      data: ValidateLocationIpRequest,
    ) => Promise<ValidateLocationIpResponse>;
    getDevicesList: () => Promise<StandaloneDevice[]>;
    getDeviceConfig: (deviceId: number | string) => Promise<string>;
    generateAuthToken: (deviceId: number | string) => Promise<StartEnrollmentResponse>;
  };
  device: {
    addDevice: (device: AddDeviceRequest) => Promise<AddDeviceResponse>;
    getDevice: (deviceId: string) => Promise<Device>;
    getDevices: () => Promise<Device[]>;
    getUserDevices: (username: string) => Promise<Device[]>;
    editDevice: (device: Device) => Promise<Device>;
    deleteDevice: (device: Device) => EmptyApiResponse;
    downloadDeviceConfig: (data: GetDeviceConfigRequest) => Promise<string>;
  };
  network: {
    addNetwork: (network: ModifyNetworkRequest['network']) => Promise<Network>;
    importNetwork: (network: ImportNetworkRequest) => Promise<ImportNetworkResponse>;
    mapUserDevices: (devices: MapUserDevicesRequest) => EmptyApiResponse;
    getNetwork: (networkId: number) => Promise<Network>;
    getNetworks: () => Promise<Network[]>;
    editNetwork: (network: ModifyNetworkRequest) => Promise<Network>;
    deleteNetwork: (networkId: number) => EmptyApiResponse;
    getOverviewStats: (data: GetNetworkStatsRequest) => Promise<OverviewStatsResponse>;
    getNetworkToken: (networkId: Network['id']) => Promise<NetworkToken>;
    getNetworkStats: (data: GetNetworkStatsRequest) => Promise<WireguardNetworkStats>;
    getGatewaysStatus: (networkId: number) => Promise<GatewayStatus[]>;
    deleteGateway: (data: DeleteGatewayRequest) => Promise<void>;
  };
  auth: {
    login: (data: LoginData) => Promise<LoginResponse>;
    logout: () => EmptyApiResponse;
    openid: {
      getOpenIdInfo: () => Promise<OpenIdInfoResponse>;
      callback: (data: CallbackData) => Promise<LoginResponse>;
    };
    mfa: {
      disable: () => EmptyApiResponse;
      enable: () => EmptyApiResponse;
      recovery: (data: RecoveryLoginRequest) => Promise<MFAFinishResponse>;
      email: {
        register: {
          start: () => EmptyApiResponse;
          finish: (data: AuthCodeRequest) => MFARecoveryCodesResponse;
        };
        disable: () => EmptyApiResponse;
        sendCode: () => EmptyApiResponse;
        verify: (data: AuthCodeRequest) => Promise<MFAFinishResponse>;
      };
      webauthn: {
        register: {
          start: (data: { name: string }) => Promise<CredentialCreationOptionsJSON>;
          finish: (data: WebAuthnRegistrationRequest) => MFARecoveryCodesResponse;
        };
        start: () => Promise<CredentialRequestOptionsJSON>;
        finish: (
          data: PublicKeyCredentialWithAssertionJSON,
        ) => Promise<MFAFinishResponse>;
        deleteKey: (data: DeleteWebAuthNKeyRequest) => EmptyApiResponse;
      };
      totp: {
        init: () => Promise<{ secret: string }>;
        enable: (data: TOTPRequest) => MFARecoveryCodesResponse;
        disable: () => EmptyApiResponse;
        verify: (data: TOTPRequest) => Promise<MFAFinishResponse>;
      };
    };
  };
  provisioning: {
    getWorkers: () => Promise<Provisioner[]>;
    deleteWorker: (id: string) => EmptyApiResponse;
    provisionYubiKey: (request_data: WorkerJobRequest) => Promise<WorkerJobResponse>;
    getJobStatus: (job_id?: number) => Promise<WorkerJobStatus>;
    getWorkerToken: () => Promise<WorkerToken>;
  };
  webhook: {
    getWebhooks: () => Promise<Webhook[]>;
    deleteWebhook: (id: string) => EmptyApiResponse;
    addWebhook: (data: Omit<Webhook, 'id'>) => EmptyApiResponse;
    changeWebhookState: (data: changeWebhookStateRequest) => EmptyApiResponse;
    editWebhook: (data: Webhook) => EmptyApiResponse;
  };
  openid: {
    getOpenidClients: () => Promise<OpenidClient[]>;
    addOpenidClient: (data: AddOpenidClientRequest) => EmptyApiResponse;
    getOpenidClient: (id: string) => Promise<OpenidClient>;
    editOpenidClient: (data: EditOpenidClientRequest) => EmptyApiResponse;
    changeOpenidClientState: (data: ChangeOpenidClientStateRequest) => EmptyApiResponse;
    deleteOpenidClient: (client_id: string) => EmptyApiResponse;
    verifyOpenidClient: (data: VerifyOpenidClientRequest) => EmptyApiResponse;
    getUserClients: (username: string) => Promise<AuthorizedClient[]>;
    removeUserClient: (data: RemoveUserClientRequest) => EmptyApiResponse;
  };
  settings: {
    getSettings: () => Promise<Settings>;
    editSettings: (data: Settings) => EmptyApiResponse;
    setDefaultBranding: (id: string) => Promise<Settings>;
    patchSettings: (data: Partial<Settings>) => EmptyApiResponse;
    getEssentialSettings: () => Promise<SettingsEssentials>;
    getEnterpriseSettings: () => Promise<SettingsEnterprise>;
    patchEnterpriseSettings: (data: Partial<SettingsEnterprise>) => EmptyApiResponse;
    testLdapSettings: () => Promise<EmptyApiResponse>;
    fetchOpenIdProviders: () => Promise<OpenIdInfo>;
    addOpenIdProvider: (data: OpenIdProvider) => Promise<EmptyApiResponse>;
    deleteOpenIdProvider: (name: string) => Promise<EmptyApiResponse>;
    editOpenIdProvider: (data: OpenIdProvider) => Promise<EmptyApiResponse>;
    testDirsync: () => Promise<DirsyncTestResponse>;
  };
  support: {
    downloadSupportData: () => Promise<unknown>;
    downloadLogs: () => Promise<string>;
  };
  mail: {
    sendTestMail: (data: TestMail) => EmptyApiResponse;
    sendSupportMail: () => EmptyApiResponse;
  };
}

export interface NavigationStore {
  isNavigationOpen: boolean;
  user?: User;
  webhook?: Webhook;
  openidclient?: OpenidClient;
  setNavigationOpen: (v: boolean) => void;
  setNavigationUser: (user: User) => void;
  setNavigationWebhook: (webhook: Webhook) => void;
  setNavigationOpenidClient: (openidclient: OpenidClient) => void;
  setState: (newState: Partial<NavigationStore>) => void;
}

export type EmptyApiResponse = AxiosPromise<unknown>;

export interface WorkerCreateJobResponse {
  id: number;
}
export interface Workers {
  [worker_name: string]: boolean;
}

export interface WorkerJobStatus {
  success?: boolean;
  errorMessage?: string;
}

export interface WorkerJobStatusError {
  message?: string;
}
export interface WorkerJobStatusRequest {
  jobId: number;
}

export interface WorkerToken {
  token: string;
}

export interface WorkerJobRequest {
  worker: string;
  username: string;
}

export interface WorkerJobResponse {
  id: number;
}

export interface KeyDetailModal {
  visible: boolean;
  user?: User;
}

export interface KeyDeleteModal {
  visible: boolean;
}

export interface ChangePasswordModal {
  visible: boolean;
  user?: User;
}

export interface ChangeUserPasswordModal {
  visible: boolean;
  user?: User;
}

export interface UserProfileStore {
  editMode: boolean;
  setEditMode: (value: boolean) => void;
}

export interface OpenidClientStore {
  editMode: boolean;
  setEditMode: (value: boolean) => void;
}

export interface AddWebhookModal {
  visible: boolean;
}

export interface EditWebhookModal {
  visible: boolean;
  webhook?: Webhook;
}

export type AddDeviceConfig = {
  network_id: number;
  network_name: string;
  config: string;
};

export interface Provisioner {
  id: string;
  connected: boolean;
  ip: string;
}

export type ModalSetter<T> = (newValues: Partial<T>) => void;

export interface StandardModalState {
  visible: boolean;
}

export interface RecoveryCodesModal extends StandardModalState {
  codes?: string[];
}

export interface WebhookModal extends StandardModalState {
  webhook?: Webhook;
}

export interface OpenIdClientModal extends StandardModalState {
  client?: OpenidClient;
  viewMode: boolean;
}

// DO NOT EXTEND THIS STORE
/**
 * this approach is outdated use individual stores instead
 */
export interface UseModalStore {
  openIdClientModal: OpenIdClientModal;
  setOpenIdClientModal: ModalSetter<OpenIdClientModal>;
  // DO NOT EXTEND THIS STORE
  keyDetailModal: KeyDetailModal;
  // DO NOT EXTEND THIS STORE
  keyDeleteModal: KeyDeleteModal;
  // DO NOT EXTEND THIS STORE
  deleteUserModal: DeleteUserModal;
  // DO NOT EXTEND THIS STORE
  toggleUserModal: ToggleUserModal;
  // DO NOT EXTEND THIS STORE
  changePasswordModal: ChangePasswordModal;
  // DO NOT EXTEND THIS STORE
  provisionKeyModal: ProvisionKeyModal;
  // DO NOT EXTEND THIS STORE
  webhookModal: WebhookModal;
  // DO NOT EXTEND THIS STORE
  addOpenidClientModal: StandardModalState;
  // DO NOT EXTEND THIS STORE
  deleteOpenidClientModal: DeleteOpenidClientModal;
  // DO NOT EXTEND THIS STORE
  enableOpenidClientModal: EnableOpenidClientModal;
  // DO NOT EXTEND THIS STORE
  manageWebAuthNKeysModal: StandardModalState;
  // DO NOT EXTEND THIS STORE
  addSecurityKeyModal: StandardModalState;
  // DO NOT EXTEND THIS STORE
  registerTOTP: StandardModalState;
  // DO NOT EXTEND THIS STORE
  recoveryCodesModal: RecoveryCodesModal;
  // DO NOT EXTEND THIS STORE
  setState: (data: Partial<UseModalStore>) => void;
  // DO NOT EXTEND THIS STORE
  setWebhookModal: ModalSetter<WebhookModal>;
  // DO NOT EXTEND THIS STORE
  setRecoveryCodesModal: ModalSetter<RecoveryCodesModal>;
  // DO NOT EXTEND THIS STORE
  setKeyDetailModal: ModalSetter<KeyDetailModal>;
  // DO NOT EXTEND THIS STORE
  setKeyDeleteModal: ModalSetter<KeyDeleteModal>;
  // DO NOT EXTEND THIS STORE
  setDeleteUserModal: ModalSetter<DeleteUserModal>;
  // DO NOT EXTEND THIS STORE
  setToggleUserModal: ModalSetter<ToggleUserModal>;
  // DO NOT EXTEND THIS STORE
  setProvisionKeyModal: ModalSetter<ProvisionKeyModal>;
  // DO NOT EXTEND THIS STORE
  setChangePasswordModal: ModalSetter<ChangePasswordModal>;
  // DO NOT EXTEND THIS STORE
  setAddOpenidClientModal: ModalSetter<StandardModalState>;
  // DO NOT EXTEND THIS STORE
  setDeleteOpenidClientModal: ModalSetter<DeleteOpenidClientModal>;
  // DO NOT EXTEND THIS STORE
  setEnableOpenidClientModal: ModalSetter<EnableOpenidClientModal>;
}

export interface UseOpenIDStore {
  openIDRedirect?: boolean;
  setOpenIDStore: (newValues: Partial<Omit<UseOpenIDStore, 'setOpenIdStore'>>) => void;
}

/**
 * full defguard instance Settings
 */
export type Settings = SettingsModules &
  SettingsSMTP &
  SettingsEnrollment &
  SettingsBranding &
  SettingsLDAP &
  SettingsOpenID &
  SettingsLicense &
  SettingsGatewayNotifications;

// essentials for core frontend, includes only those that are required for frontend operations
export type SettingsEssentials = SettingsModules & SettingsBranding;

export type SettingsEnrollment = {
  enrollment_vpn_step_optional: boolean;
  enrollment_welcome_message: string;
  enrollment_welcome_email: string;
  enrollment_welcome_email_subject: string;
  enrollment_use_welcome_message_as_email: boolean;
};

export type SettingsSMTP = {
  smtp_server?: string;
  smtp_port?: number;
  smtp_encryption: string;
  smtp_user?: string;
  smtp_password?: string;
  smtp_sender?: string;
};

export type SettingsModules = {
  openid_enabled: boolean;
  wireguard_enabled: boolean;
  webhooks_enabled: boolean;
  worker_enabled: boolean;
};

export type SettingsBranding = {
  instance_name: string;
  main_logo_url: string;
  nav_logo_url: string;
};

export type SettingsLDAP = {
  ldap_bind_password?: string;
  ldap_bind_username?: string;
  ldap_url?: string;
  ldap_group_member_attr: string;
  ldap_group_obj_class: string;
  ldap_group_search_base: string;
  ldap_groupname_attr: string;
  ldap_member_attr: string;
  ldap_user_obj_class: string;
  ldap_user_search_base: string;
  ldap_username_attr: string;
};

export type SettingsOpenID = {
  openid_create_account: boolean;
};

export type SettingsLicense = {
  license: string;
};

export type SettingsGatewayNotifications = {
  gateway_disconnect_notifications_enabled: boolean;
  gateway_disconnect_notifications_inactivity_threshold: number;
  gateway_disconnect_notifications_reconnect_notification_enabled: boolean;
};

export type SettingsEnterprise = {
  admin_device_management: boolean;
  disable_all_traffic: boolean;
  only_client_activation: boolean;
};

export type EnterpriseLicenseInfo = {
  valid_until?: string;
  subscription: boolean;
};

export type EnterpriseStatus = {
  enabled: boolean;
};

export type EnterpriseInfo = {
  expired: boolean;
  limits_exceeded: boolean;
  subscription: boolean;
  // iso utc date
  valid_until: string;
};

export interface Webhook {
  id: string;
  url: string;
  description: string;
  token: string;
  enabled: boolean;
  on_user_created: boolean;
  on_user_deleted: boolean;
  on_user_modified: boolean;
  on_hwkey_provision: boolean;
}

export interface OpenidClient {
  id: string;
  name: string;
  client_id: string;
  client_secret: string;
  redirect_uri: string[];
  scope: string[];
  enabled: boolean;
}

export interface OpenIdInfo {
  settings: {
    create_account: boolean;
  };
  provider?: OpenIdProvider;
}

export interface OpenIdProvider {
  id: number;
  name: string;
  base_url: string;
  client_id: string;
  client_secret: string;
  display_name: string;
  google_service_account_key?: string;
  google_service_account_email?: string;
  admin_email?: string;
  directory_sync_enabled: boolean;
  directory_sync_interval: number;
  directory_sync_user_behavior: 'keep' | 'disable' | 'delete';
  directory_sync_admin_behavior: 'keep' | 'disable' | 'delete';
  directory_sync_target: 'all' | 'users' | 'groups';
  okta_private_jwk?: string;
  okta_dirsync_client_id?: string;
}

export interface EditOpenidClientRequest {
  id: string;
  name: string;
  client_id: string;
  client_secret: string;
  redirect_uri: string[];
  enabled: boolean;
}

export interface AddOpenidClientRequest {
  name: string;
  redirect_uri: string[];
  enabled: boolean;
  scope: string[];
}

export interface changeWebhookStateRequest {
  id: string;
  enabled: boolean;
}

export interface ChangeOpenidClientStateRequest {
  clientId: string;
  enabled: boolean;
}

export interface VerifyOpenidClientRequest {
  client_id: string;
  scope: string;
  redirect_uri: string;
  response_type: string;
  state: string;
  nonce: string;
  allow: boolean;
}

export interface AuthorizedClient {
  id: string;
  username: string;
  client_id: string;
  home_url: string;
  date: string;
}

export enum OverviewLayoutType {
  GRID = 'GRID',
  LIST = 'LIST',
  MAP = 'MAP',
}

export interface OverviewStore {
  viewMode: OverviewLayoutType;
  defaultViewMode: OverviewLayoutType;
  statsFilter: number;
  networks?: Network[];
  selectedNetworkId?: number;
  setState: (override: Partial<OverviewStore>) => void;
}

export interface NetworkSpeedStats {
  collected_at: string;
  download: number;
  upload: number;
}

export interface NetworkDeviceStats {
  connected_at: string;
  id: number;
  name: string;
  public_ip: string;
  wireguard_ip: string;
  stats: NetworkSpeedStats[];
}

export type OverviewStatsResponse = {
  user_devices: NetworkUserStats[];
  network_devices: StandaloneDeviceStats[];
};

export type StandaloneDeviceStats = {
  id: number;
  stats: NetworkSpeedStats[];
  user_id: number;
  name: string;
  wireguard_ip?: string;
  public_ip?: string;
  connected_at?: string;
};

export interface NetworkUserStats {
  user: User;
  devices: NetworkDeviceStats[];
}

export interface WireguardNetworkStats {
  active_users: number;
  active_devices: number;
  current_active_users: number;
  current_active_devices: number;
  upload: number;
  download: number;
  transfer_series: NetworkSpeedStats[];
}

export interface TOTPRequest {
  code: string;
}

export interface WebAuthnRegistrationRequest {
  name: string;
  rpkc: PublicKeyCredentialWithAttestationJSON;
}

export interface RemoveUserClientRequest {
  username: string;
  client_id: string;
}

export interface TestMail {
  to: string;
}

export type SMTPError = AxiosError<{ error: string }>;

export type Group = string;

export type GroupInfo = {
  name: string;
  members: string[];
  vpn_locations: string[];
  is_admin: boolean;
};

export type DirsyncTestResponse = {
  message: string;
  success: boolean;
};

export type CreateStandaloneDeviceRequest = {
  name: string;
  location_id: number;
  assigned_ip: string;
  wireguard_pubkey?: string;
  description?: string;
};

export type ValidateLocationIpRequest = {
  ip: string;
  location: number | string;
};

export type ValidateLocationIpResponse = {
  available: boolean;
  valid: boolean;
};

export type GetAvailableLocationIpRequest = {
  locationId: number | string;
};

export type GetAvailableLocationIpResponse = {
  ip: string;
  network_part: string;
  modifiable_part: string;
  network_prefix: string;
};

export type StandaloneDevice = {
  id: number;
  name: string;
  assigned_ip: string;
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
  split_ip: {
    network_part: string;
    modifiable_part: string;
    network_prefix: string;
  };
};

export type DeviceConfigurationResponse = {
  address: string;
  allowed_ips: string[];
  config: string;
  endpoint: string;
  keepalive_interval: number;
  mfa_enabled: boolean;
  network_id: number;
  network_name: string;
  pubkey: string;
};

export type CreateStandaloneDeviceResponse = {
  config: DeviceConfigurationResponse;
  device: StandaloneDevice;
};

export type StandaloneDeviceEditRequest = {
  id: number;
  assigned_ip: string;
  description?: string;
  name: string;
};

export type LicenseLimits = {
  user: boolean;
  device: boolean;
  wireguard_network: boolean;
};

export type LicenseInfo = {
  enterprise: boolean;
  limits_exceeded: LicenseLimits;
  any_limit_exceeded: boolean;
  is_enterprise_free: boolean;
};
