import {
  CredentialCreationOptionsJSON,
  CredentialRequestOptionsJSON,
  PublicKeyCredentialWithAssertionJSON,
  PublicKeyCredentialWithAttestationJSON,
} from '@github/webauthn-json';
import { AxiosPromise } from 'axios';

import { Locales } from '../i18n/i18n-types';
import { AddDeviceSetupChoice } from '../pages/users/UserProfile/UserDevices/modals/AddUserDeviceModal/steps/SetupStep';

export enum UserStatus {
  active = 'Active',
  inactive = 'Inactive',
  awaitingLogin = 'Awaiting login',
}

export enum UserMFAMethod {
  NONE = 'None',
  ONE_TIME_PASSWORD = 'OneTimePassword',
  WEB_AUTH_N = 'Webauthn',
  WEB3 = 'Web3',
}

export interface User {
  username: string;
  last_name: string;
  first_name: string;
  devices: Device[];
  wallets: WalletInfo[];
  security_keys: SecurityKey[];
  mfa_method: UserMFAMethod;
  mfa_enabled: boolean;
  totp_enabled: boolean;
  email?: string;
  phone?: string;
  lastConnected?: Date;
  lastLocation?: Location;
  lastLocations?: Location[];
  status?: UserStatus;
  pgp_cert_id?: string;
  pgp_key?: string;
  ssh_key?: string;
  groups: string[];
  authorized_apps?: OAuth2AuthorizedApps[];
}

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

export interface WalletInfo {
  address: string;
  chain_id: number;
  name: string;
  use_for_mfa: boolean;
}

export interface Device {
  id: string;
  name: string;
  wireguard_ip: string;
  wireguard_pubkey: string;
  config: string;
  created: string;
}
export interface AddDeviceRequest {
  username: string;
  name: string;
  wireguard_pubkey: string;
}

export interface Network {
  id: string;
  name: string;
  address: string;
  port: number;
  endpoint: string;
  connected_at?: string;
  allowed_ips?: string[];
  dns?: string;
}

export interface ModifyNetworkRequest
  extends Omit<Network, 'id' | 'connected_at' | 'allowed_ips'> {
  allowed_ips: string;
}

export interface ImportNetworkRequest {
  name: string;
  endpoint: string;
  config: string;
}

export interface NetworkToken {
  token: string;
}

export interface LoginData {
  username: string;
  password: string;
}

export interface AuthStore {
  user?: User;
  isAdmin?: boolean;
  setState: (newState: Partial<AuthStore>) => void;
  logIn: (user: User) => void;
  logOut: () => void;
  // This is used in OAuth / OpenId flows to return into originally requested url after auth is completed.
  authLocation?: string;
}

export interface DeleteUserModal {
  visible: boolean;
  user?: User;
}

export interface ProvisionKeyModal {
  visible: boolean;
  user?: User;
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

export interface WalletChallengeRequest {
  name?: string;
  username: string;
  address: string;
  chainId?: number;
}

export interface WalletChallenge {
  id: number;
  message: string;
}

export interface AddWalletRequest {
  name: string;
  chain_id: number;
  username: string;
  address: string;
  signature: string;
}

export interface AddUserRequest {
  username: string;
  password: string;
  email: string;
  last_name: string;
  first_name: string;
  phone: string;
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
}

export interface UserEditRequest {
  username: string;
  data: Partial<User>;
}

export interface EditWalletMFARequest {
  username: string;
  address: string;
  use_for_mfa: boolean;
}

export interface MFALoginResponse {
  mfa_method: UserMFAMethod;
  totp_available: boolean;
  web3_available: boolean;
  webauthn_available: boolean;
}

export interface LoginResponse {
  url?: string;
  user?: User;
  mfa?: MFALoginResponse;
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

export interface ConnectionInfo {
  connected: boolean;
}

export interface MFAFinishResponse {
  url?: string;
  user?: User;
}

export interface ApiHook {
  getVersion: () => Promise<VersionResponse>;
  oAuth: {
    consent: (params: unknown) => Promise<EmptyApiResponse>;
  };
  groups: {
    getGroups: () => Promise<GroupsResponse>;
  };
  user: {
    getMe: () => Promise<User>;
    addUser: (data: AddUserRequest) => EmptyApiResponse;
    getUser: (username: string) => Promise<User>;
    getUsers: () => Promise<User[]>;
    editUser: (data: UserEditRequest) => Promise<User>;
    deleteUser: (user: User) => EmptyApiResponse;
    usernameAvailable: (username: string) => EmptyApiResponse;
    changePassword: (data: ChangePasswordRequest) => EmptyApiResponse;
    walletChallenge: (data: WalletChallengeRequest) => Promise<WalletChallenge>;
    setWallet: (data: AddWalletRequest) => EmptyApiResponse;
    deleteWallet: (data: WalletChallengeRequest) => EmptyApiResponse;
    addToGroup: (data: UserGroupRequest) => EmptyApiResponse;
    removeFromGroup: (data: UserGroupRequest) => EmptyApiResponse;
  };
  device: {
    addDevice: (device: AddDeviceRequest) => Promise<string>;
    getDevice: (deviceId: string) => Promise<Device>;
    getDevices: () => Promise<Device[]>;
    getUserDevices: (username: string) => Promise<Device[]>;
    editDevice: (device: Device) => Promise<Device>;
    deleteDevice: (device: Device) => EmptyApiResponse;
    downloadDeviceConfig: (id: string) => Promise<string>;
  };
  network: {
    addNetwork: (network: ModifyNetworkRequest) => Promise<Network>;
    getNetwork: (networkId: string) => Promise<Network>;
    getNetworks: () => Promise<Network[]>;
    editNetwork: (network: ModifyNetworkRequest) => Promise<Network>;
    deleteNetwork: (network: Network) => EmptyApiResponse;
    getUsersStats: (
      data?: GetNetworkStatsRequest
    ) => Promise<NetworkUserStats[]>;
    getNetworkToken: (networkId: string) => Promise<NetworkToken>;
    getNetworkStats: (
      data?: GetNetworkStatsRequest
    ) => Promise<WireguardNetworkStats>;
    getGatewayStatus: () => Promise<ConnectionInfo>;
  };
  auth: {
    login: (data: LoginData) => Promise<LoginResponse>;
    logout: () => EmptyApiResponse;
    mfa: {
      disable: () => EmptyApiResponse;
      enable: () => EmptyApiResponse;
      recovery: (data: RecoveryLoginRequest) => Promise<MFAFinishResponse>;
      webauthn: {
        register: {
          start: (data: {
            name: string;
          }) => Promise<CredentialCreationOptionsJSON>;
          finish: (
            data: WebAuthnRegistrationRequest
          ) => MFARecoveryCodesResponse;
        };
        start: () => Promise<CredentialRequestOptionsJSON>;
        finish: (
          data: PublicKeyCredentialWithAssertionJSON
        ) => Promise<MFAFinishResponse>;
        deleteKey: (data: DeleteWebAuthNKeyRequest) => EmptyApiResponse;
      };
      totp: {
        init: () => Promise<{ secret: string }>;
        enable: (data: TOTPRequest) => MFARecoveryCodesResponse;
        disable: () => EmptyApiResponse;
        verify: (data: TOTPRequest) => Promise<MFAFinishResponse>;
      };
      web3: {
        start: (data: Web3StartRequest) => Promise<{ challenge: string }>;
        finish: (data: WalletSignature) => Promise<MFAFinishResponse>;
        updateWalletMFA: (
          data: EditWalletMFARequest
        ) => MFARecoveryCodesResponse;
      };
    };
  };
  provisioning: {
    getWorkers: () => Promise<Provisioner[]>;
    deleteWorker: (id: string) => EmptyApiResponse;
    provisionYubiKey: (
      request_data: WorkerJobRequest
    ) => Promise<WorkerJobResponse>;
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
    changeOpenidClientState: (
      data: ChangeOpenidClientStateRequest
    ) => EmptyApiResponse;
    deleteOpenidClient: (client_id: string) => EmptyApiResponse;
    verifyOpenidClient: (data: VerifyOpenidClientRequest) => EmptyApiResponse;
    getUserClients: (username: string) => Promise<AuthorizedClient[]>;
    removeUserClient: (data: RemoveUserClientRequest) => EmptyApiResponse;
  };
  license: {
    getLicense: () => Promise<License>;
  };
  settings: {
    getSettings: () => Promise<Settings>;
    editSettings: (data: Settings) => EmptyApiResponse;
    setDefaultBranding: (id: string) => Promise<Settings>;
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
}

export interface SelectOption<T> {
  label: string;
  value: T;
}

export type EmptyApiResponse = AxiosPromise<unknown>;

export interface WorkerCreateJobResponse {
  id: number;
}
export interface Workers {
  [worker_name: string]: boolean;
}

export interface WorkerJobStatus {
  pgp_cert_id?: string;
  pgp_key?: string;
  ssh_key?: string;
  success?: boolean;
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

export interface ChangeWalletModal {
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

interface ModalStepsState {
  currentStep: number;
  endStep: number;
  nextStep: () => void;
}

export interface UserDeviceModal extends StandardModalState, ModalStepsState {
  config?: string;
  deviceName?: string;
  choice?: AddDeviceSetupChoice;
}

export interface Provisioner {
  id: string;
  connected: boolean;
  ip: string;
}

export type ModalSetter<T> = (newValues: Partial<T>) => void;

export interface StandardModalState {
  visible: boolean;
}

export interface DeleteUserDeviceModal extends StandardModalState {
  device?: Device;
}

export interface RecoveryCodesModal extends StandardModalState {
  codes?: string[];
}

export interface ConnectWalletModal extends StandardModalState {
  onConnect?: () => void;
}

export interface EditUserDeviceModal extends StandardModalState {
  device?: Device;
}

export interface WebhookModal extends StandardModalState {
  webhook?: Webhook;
}

export interface OpenIdClientModal extends StandardModalState {
  client?: OpenidClient;
  viewMode: boolean;
}

export interface UseModalStore {
  openIdClientModal: OpenIdClientModal;
  setOpenIdClientModal: ModalSetter<OpenIdClientModal>;
  addDeviceDesktopModal: StandardModalState;
  editUserDeviceModal: EditUserDeviceModal;
  addWalletModal: StandardModalState;
  keyDetailModal: KeyDetailModal;
  keyDeleteModal: KeyDeleteModal;
  deleteUserModal: DeleteUserModal;
  addUserModal: StandardModalState;
  licenseModal: StandardModalState;
  changePasswordModal: ChangePasswordModal;
  changeWalletModal: ChangeWalletModal;
  provisionKeyModal: ProvisionKeyModal;
  webhookModal: WebhookModal;
  addOpenidClientModal: StandardModalState;
  deleteOpenidClientModal: DeleteOpenidClientModal;
  enableOpenidClientModal: EnableOpenidClientModal;
  userDeviceModal: UserDeviceModal;
  deleteUserDeviceModal: DeleteUserDeviceModal;
  manageWebAuthNKeysModal: StandardModalState;
  addSecurityKeyModal: StandardModalState;
  registerTOTP: StandardModalState;
  connectWalletModal: ConnectWalletModal;
  recoveryCodesModal: RecoveryCodesModal;
  setState: (data: Partial<UseModalStore>) => void;
  setWebhookModal: ModalSetter<WebhookModal>;
  setRecoveryCodesModal: ModalSetter<RecoveryCodesModal>;
  setDeleteUserDeviceModal: ModalSetter<DeleteUserDeviceModal>;
  setUserDeviceModal: ModalSetter<UserDeviceModal>;
  setAddUserModal: ModalSetter<StandardModalState>;
  setKeyDetailModal: ModalSetter<KeyDetailModal>;
  setKeyDeleteModal: ModalSetter<KeyDeleteModal>;
  setDeleteUserModal: ModalSetter<DeleteUserModal>;
  setProvisionKeyModal: ModalSetter<ProvisionKeyModal>;
  setChangePasswordModal: ModalSetter<ChangePasswordModal>;
  setChangeWalletModal: ModalSetter<ChangeWalletModal>;
  setAddOpenidClientModal: ModalSetter<StandardModalState>;
  setDeleteOpenidClientModal: ModalSetter<DeleteOpenidClientModal>;
  setEnableOpenidClientModal: ModalSetter<EnableOpenidClientModal>;
  setLicenseModal: ModalSetter<StandardModalState>;
}

export interface UseAppStore {
  backendVersion?: string;
  settings?: Settings;
  license?: License;
  version?: string;
  language?: Locales;
  setAppStore: (newValues: Partial<Omit<UseAppStore, 'setAppStore'>>) => void;
}

export interface UseOpenIDStore {
  openIDRedirect?: boolean;
  setOpenIDStore: (
    newValues: Partial<Omit<UseOpenIDStore, 'setOpenIdStore'>>
  ) => void;
}

export interface Settings {
  id: string;
  challenge_template: string;
  web3_enabled: boolean;
  openid_enabled: boolean;
  oauth_enabled: boolean;
  ldap_enabled: boolean;
  wireguard_enabled: boolean;
  webhooks_enabled: boolean;
  worker_enabled: boolean;
  main_logo_url: string;
  nav_logo_url: string;
  instance_name: string;
}

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

export interface License {
  company: string;
  expiration: Date;
  oauth: boolean;
  enterprise: boolean;
  openid: boolean;
  ldap: boolean;
  worker: boolean;
}

export interface WalletProvider {
  title: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Icon: any;
  right: JSX.Element | string | null;
  active?: boolean;
}

export interface WalletSignature {
  address: string;
  signature: string;
}

export interface Web3StartRequest {
  address: string;
}

export interface TOTPRequest {
  code: number;
}
export interface WebAuthnRegistrationRequest {
  name: string;
  rpkc: PublicKeyCredentialWithAttestationJSON;
}
export interface RemoveUserClientRequest {
  username: string;
  client_id: string;
}
