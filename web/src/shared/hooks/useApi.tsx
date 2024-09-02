import axios, { AxiosResponse } from 'axios';
import { useEffect, useMemo } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import {
  AddOpenidClientRequest,
  AddUserRequest,
  AddWalletRequest,
  ApiError,
  ApiHook,
  AuthorizedClient,
  ChangeOpenidClientStateRequest,
  ChangePasswordRequest,
  changeWebhookStateRequest,
  Device,
  EditOpenidClientRequest,
  EmptyApiResponse,
  GetNetworkStatsRequest,
  GroupsResponse,
  LoginData,
  MFALoginResponse,
  Network,
  NetworkToken,
  NetworkUserStats,
  OpenidClient,
  OpenIdProvider,
  RemoveUserClientRequest,
  ResetPasswordRequest,
  Settings,
  StartEnrollmentRequest,
  StartEnrollmentResponse,
  User,
  UserEditRequest,
  UserGroupRequest,
  VerifyOpenidClientRequest,
  WalletChallenge,
  WalletChallengeRequest,
  WireguardNetworkStats,
  WorkerJobRequest,
  WorkerJobResponse,
  WorkerJobStatus,
  WorkerToken,
} from '../types';
import { removeNulls } from '../utils/removeNulls';
import { useToaster } from './useToaster';

interface HookProps {
  baseURL?: string;
  // Spawns toaster type Error when request response code is above 399
  notifyError?: boolean;
}

const envBaseUrl = import.meta.env.API_BASE_URL;

const unpackRequest = <T,>(res: AxiosResponse<T>): T => res.data;

const useApi = (props?: HookProps): ApiHook => {
  const toaster = useToaster();
  const { LL } = useI18nContext();

  const client = useMemo(() => {
    const res = axios.create({
      baseURL: envBaseUrl && String(envBaseUrl).length > 0 ? envBaseUrl : '/api/v1',
    });

    res.defaults.headers.common['Content-Type'] = 'application/json';
    return res;
  }, []);

  if (props) {
    const { baseURL } = props;
    if (baseURL && baseURL.length) {
      client.defaults.baseURL = baseURL;
    }
  }

  const addUser = async (data: AddUserRequest) => {
    return client.post<User>(`/user`, data).then((res) => res.data);
  };

  const getMe = () => client.get<User>(`/me`).then((res) => res.data);

  const getUser: ApiHook['user']['getUser'] = async (username) =>
    client.get(`/user/${username}`).then((res) => res.data);

  const editUser = async ({ username, data }: UserEditRequest) =>
    client.put<User>(`/user/${username}`, data).then(unpackRequest);

  const deleteUser = async (user: User) =>
    client.delete<EmptyApiResponse>(`/user/${user.username}`).then((res) => res.data);

  const fetchDevices = async () =>
    client.get<Device[]>(`/device`).then((res) => res.data);

  const fetchDevice = async (id: string) =>
    client.get<Device>(`/device/${id}`).then((res) => res.data);

  const getUsers = () => client.get('/user').then(unpackRequest);

  const downloadDeviceConfig: ApiHook['device']['downloadDeviceConfig'] = async (data) =>
    client
      .get<string>(`/network/${data.network_id}/device/${data.device_id}/config`)
      .then((res) => res.data);

  const modifyDevice = async (device: Device) =>
    client.put<Device>(`/device/${device.id}`, device).then((res) => res.data);

  const deleteDevice = async (device: Device) =>
    client.delete<EmptyApiResponse>(`/device/${device.id}`);

  const addDevice: ApiHook['device']['addDevice'] = async ({ username, ...rest }) =>
    client.post(`/device/${username}`, rest).then((res) => res.data);

  const fetchUserDevices = async (username: string) =>
    client.get<Device[]>(`/device/user/${username}`).then((res) => res.data);

  const fetchNetworks = async () =>
    client.get<Network[]>(`/network`).then((res) => res.data);

  const fetchNetwork = async (id: number) =>
    client.get<Network>(`/network/${id}`).then((res) => res.data);

  // For now there is only one network
  const modifyNetwork: ApiHook['network']['editNetwork'] = async (data) =>
    client.put<Network>(`/network/${data.id}`, data.network).then((res) => res.data);

  const deleteNetwork: ApiHook['network']['deleteNetwork'] = async (id) =>
    client.delete<EmptyApiResponse>(`/network/${id}`);

  const addNetwork: ApiHook['network']['addNetwork'] = (network) =>
    client.post(`/network`, network).then(unpackRequest);

  const importNetwork: ApiHook['network']['importNetwork'] = (network) =>
    client.post(`/network/import`, network).then(unpackRequest);

  const mapUserDevices: ApiHook['network']['mapUserDevices'] = (data) =>
    client
      .post(`/network/${data.networkId}/devices`, { devices: data.devices })
      .then(unpackRequest);

  const login: ApiHook['auth']['login'] = (data: LoginData) =>
    client.post('/auth', data).then((response) => {
      if (response.status === 200) {
        return response.data;
      }
      if (response.status === 201) {
        return {
          mfa: response.data as MFALoginResponse,
        };
      }
      return {};
    });

  const logout = () => client.post<EmptyApiResponse>('/auth/logout').then(unpackRequest);

  const getOpenidInfo: ApiHook['auth']['openid']['getOpenIdInfo'] = () =>
    client.get(`/openid/auth_info`).then(unpackRequest);

  const usernameAvailable = (username: string) =>
    client.post('/user/available', { username });

  const getWorkers = () => client.get('/worker').then((res) => res.data);

  const provisionYubiKey = (data: WorkerJobRequest) =>
    client.post<WorkerJobResponse>(`/worker/job`, data).then((response) => response.data);

  const getJobStatus = (id?: number) =>
    client.get<WorkerJobStatus>(`/worker/${id}`).then((res) => res.data);

  const changePassword = ({ username, ...rest }: ChangePasswordRequest) =>
    client.put<EmptyApiResponse>(`/user/${username}/password`, rest);

  const resetPassword = ({ username }: ResetPasswordRequest) =>
    client.post<EmptyApiResponse>(`/user/${username}/reset_password`);

  const startEnrollment = ({ username, ...rest }: StartEnrollmentRequest) =>
    client
      .post<StartEnrollmentResponse>(`/user/${username}/start_enrollment`, rest)
      .then((response) => response.data);

  const walletChallenge = ({
    username,
    address,
    name,
    chainId,
  }: WalletChallengeRequest) =>
    client
      .get<WalletChallenge>(
        `/user/${username}/challenge?address=${address}&name=${name}&chain_id=${chainId}`,
      )
      .then((response) => response.data);

  const setWallet = ({ username, ...rest }: AddWalletRequest) =>
    client
      .put<EmptyApiResponse>(`/user/${username}/wallet`, rest)
      .then((response) => response.data);

  const deleteWallet = ({ username, address }: WalletChallengeRequest) =>
    client
      .delete<EmptyApiResponse>(`/user/${username}/wallet/${address}`)
      .then((response) => response.data);

  const getGroups = () => client.get<GroupsResponse>('/group').then(unpackRequest);

  const addToGroup = ({ group, ...rest }: UserGroupRequest) =>
    client.post<EmptyApiResponse>(`/group/${group}`, rest);

  const removeFromGroup = ({ group, username }: UserGroupRequest) =>
    client.delete(`/group/${group}/user/${username}`);

  const createGroup: ApiHook['groups']['createGroup'] = async (data) =>
    client.post(`/group`, data).then(unpackRequest);

  const editGroup: ApiHook['groups']['editGroup'] = async ({ originalName, ...rest }) =>
    client.put(`/group/${originalName}`, rest).then(unpackRequest);

  const deleteWorker = (id: string) =>
    client.delete<EmptyApiResponse>(`/worker/${id}`).then((res) => res.data);

  const getWebhooks = () => client.get('/webhook').then((res) => res.data);

  const deleteWebhook = (id: string) =>
    client.delete<EmptyApiResponse>(`/webhook/${id}`).then((res) => res.data);

  const changeWebhookState = ({ id, ...rest }: changeWebhookStateRequest) =>
    client.post<EmptyApiResponse>(`/webhook/${id}`, rest);

  const addWebhook: ApiHook['webhook']['addWebhook'] = async (data) => {
    return client.post<EmptyApiResponse>('/webhook', data);
  };
  const editWebhook: ApiHook['webhook']['editWebhook'] = async ({ id, ...rest }) => {
    return client.put<EmptyApiResponse>(`/webhook/${id}`, rest);
  };
  const getOpenidClients = () => client.get('/oauth').then((res) => res.data);

  const getOpenidClient = async (client_id: string) =>
    client.get<OpenidClient>(`/oauth/${client_id}`).then((res) => res.data);

  const addOpenidClient = async (data: AddOpenidClientRequest) => {
    return client.post<EmptyApiResponse>('/oauth', data);
  };
  const editOpenidClient = async ({ client_id, ...rest }: EditOpenidClientRequest) => {
    return client.put<EmptyApiResponse>(`/oauth/${client_id}`, rest);
  };
  const changeOpenidClientState = async ({
    clientId,
    ...rest
  }: ChangeOpenidClientStateRequest) => {
    return client.post<EmptyApiResponse>(`/oauth/${clientId}`, rest);
  };
  const deleteOpenidClient = async (id: string) =>
    client.delete<EmptyApiResponse>(`/oauth/${id}`).then((res) => res.data);

  const verifyOpenidClient = async (data: VerifyOpenidClientRequest) =>
    client.post('openid/verify', data);

  const getUserClients = async (username: string) =>
    client.get<AuthorizedClient[]>(`/oauth/apps/${username}`).then((res) => res.data);

  const removeUserClient = async (data: RemoveUserClientRequest) =>
    client
      .delete<EmptyApiResponse>(`/user/${data.username}/oauth_app/${data.client_id}`)
      .then((res) => res.data);

  const oAuthConsent = (params: unknown) =>
    client
      .post('/oauth/authorize', null, {
        params: params,
      })
      .then((res) => res.data);

  const getUsersStats = (data: GetNetworkStatsRequest) =>
    client
      .get<NetworkUserStats[]>(`/network/${data.id}/stats/users`, {
        params: {
          ...data,
        },
      })
      .then(unpackRequest);

  const getNetworkToken: ApiHook['network']['getNetworkToken'] = (networkId) =>
    client.get<NetworkToken>(`/network/${networkId}/token`).then(unpackRequest);

  const getNetworkStats: ApiHook['network']['getNetworkStats'] = (data) =>
    client
      .get<WireguardNetworkStats>(`/network/${data.id}/stats`, {
        params: {
          ...data,
        },
      })
      .then(unpackRequest);

  const getWorkerToken = () =>
    client.get<WorkerToken>('/worker/token').then(unpackRequest);

  const mfaDisable = () => client.delete('/auth/mfa').then(unpackRequest);

  // eslint-disable-next-line max-len
  const mfaWebauthnRegisterStart: ApiHook['auth']['mfa']['webauthn']['register']['start'] =
    () => client.post('/auth/webauthn/init').then(unpackRequest);

  // eslint-disable-next-line max-len
  const mfaWebauthnRegisterFinish: ApiHook['auth']['mfa']['webauthn']['register']['finish'] =
    async (data) => client.post('/auth/webauthn/finish', data).then(unpackRequest);

  const mfaWebauthnStart = () => client.post('/auth/webauthn/start').then(unpackRequest);

  const mfaWebautnFinish: ApiHook['auth']['mfa']['webauthn']['finish'] = (data) =>
    client.post('/auth/webauthn', data).then(unpackRequest);

  const mfaTOTPInit = () => client.post('/auth/totp/init').then(unpackRequest);

  const mfaTOTPEnable: ApiHook['auth']['mfa']['totp']['enable'] = (data) =>
    client.post('/auth/totp', data).then(unpackRequest);

  const mfaTOTPDisable = () => client.delete('/auth/totp').then(unpackRequest);

  const mfaTOTPVerify: ApiHook['auth']['mfa']['totp']['verify'] = (data) =>
    client.post('/auth/totp/verify', data).then(unpackRequest);

  const mfaEmailMFAInit: ApiHook['auth']['mfa']['email']['register']['start'] = () =>
    client.post('/auth/email/init').then(unpackRequest);

  const mfaEmailMFAEnable: ApiHook['auth']['mfa']['email']['register']['finish'] = (
    data,
  ) => client.post('/auth/email', data).then(unpackRequest);

  const mfaEmailMFADisable = () => client.delete('/auth/email').then(unpackRequest);

  const mfaEmailMFASendCode: ApiHook['auth']['mfa']['email']['sendCode'] = () =>
    client.get('/auth/email').then(unpackRequest);

  const mfaEmailMFAVerify: ApiHook['auth']['mfa']['email']['verify'] = (data) =>
    client.post('/auth/email/verify', data).then(unpackRequest);

  const mfaWeb3Start: ApiHook['auth']['mfa']['web3']['start'] = (data) =>
    client.post('/auth/web3/start', data).then(unpackRequest);

  const mfaWeb3Finish: ApiHook['auth']['mfa']['web3']['finish'] = (data) =>
    client.post('/auth/web3', data).then(unpackRequest);

  const editWalletMFA: ApiHook['auth']['mfa']['web3']['updateWalletMFA'] = ({
    address,
    username,
    ...rest
  }) =>
    client
      .put(`/user/${username}/wallet/${address}`, {
        ...rest,
      })
      .then(unpackRequest);

  const mfaWebauthnDeleteKey: ApiHook['auth']['mfa']['webauthn']['deleteKey'] = ({
    keyId,
    username,
  }) => client.delete(`/user/${username}/security_key/${keyId}`);

  const getSettings = () => client.get('/settings').then(unpackRequest);

  const editSettings = async (settings: Settings) =>
    client.put('/settings', settings).then(unpackRequest);

  const getEnterpriseStatus = () => client.get('/enterprise_status').then(unpackRequest);

  const mfaEnable = () => client.put('/auth/mfa').then(unpackRequest);

  const recovery: ApiHook['auth']['mfa']['recovery'] = (data) =>
    client.post('/auth/recovery', data).then(unpackRequest);

  const getAppInfo: ApiHook['getAppInfo'] = () => client.get('/info').then(unpackRequest);

  const setDefaultBranding: ApiHook['settings']['setDefaultBranding'] = (id: string) =>
    client.put(`/settings/${id}`).then(unpackRequest);

  const downloadSupportData: ApiHook['support']['downloadSupportData'] = async () =>
    client.get<unknown>(`/support/configuration`).then((res) => res.data);

  const downloadLogs: ApiHook['support']['downloadLogs'] = async () =>
    client.get<string>(`/support/logs`).then((res) => res.data);

  const getGatewaysStatus: ApiHook['network']['getGatewaysStatus'] = (networkId) =>
    client.get(`/network/${networkId}/gateways`).then(unpackRequest);

  const deleteGateway: ApiHook['network']['deleteGateway'] = (data) =>
    client.delete(`/network/${data.networkId}/gateways/${data.gatewayId}`);

  const changePasswordSelf: ApiHook['changePasswordSelf'] = (data) =>
    client.put('/user/change_password', data).then(unpackRequest);

  const sendTestMail: ApiHook['mail']['sendTestMail'] = (data) =>
    client.post('/mail/test', data).then(unpackRequest);

  const sendSupportMail: ApiHook['mail']['sendSupportMail'] = () =>
    client.post('/mail/support', {}).then(unpackRequest);

  const startDesktopActivation: ApiHook['user']['startDesktopActivation'] = (data) =>
    client.post(`/user/${data.username}/start_desktop`, data).then(unpackRequest);

  const getAuthenticationKeysInfo: ApiHook['user']['getAuthenticationKeysInfo'] = (
    data,
  ) => client.get(`/user/${data.username}/auth_key`).then(unpackRequest);

  const addAuthenticationKey: ApiHook['user']['addAuthenticationKey'] = (data) =>
    client.post(`/user/${data.username}/auth_key`, data).then(unpackRequest);

  const renameAuthenticationKey: ApiHook['user']['renameAuthenticationKey'] = (data) =>
    client
      .post(`/user/${data.username}/auth_key/${data.id}/rename`, {
        name: data.name,
      })
      .then(unpackRequest);

  const deleteAuthenticationKey: ApiHook['user']['deleteAuthenticationKey'] = (data) =>
    client.delete(`/user/${data.username}/auth_key/${data.id}`).then(unpackRequest);

  const renameYubikey: ApiHook['user']['renameYubikey'] = (data) =>
    client
      .post(`/user/${data.username}/yubikey/${data.id}/rename`, {
        name: data.name,
      })
      .then(unpackRequest);

  const deleteYubiKey: ApiHook['user']['deleteYubiKey'] = (data) =>
    client.delete(`/user/${data.username}/yubikey/${data.id}`).then(unpackRequest);

  const patchSettings: ApiHook['settings']['patchSettings'] = (data) =>
    client.patch('/settings', data).then(unpackRequest);

  const getEssentialSettings: ApiHook['settings']['getEssentialSettings'] = () =>
    client.get('/settings_essentials').then(unpackRequest);

  const getEnterpriseSettings: ApiHook['settings']['getEnterpriseSettings'] = () =>
    client.get('/settings_enterprise').then(unpackRequest);

  const patchEnterpriseSettings: ApiHook['settings']['patchEnterpriseSettings'] = (
    data,
  ) => client.patch('/settings_enterprise', data).then(unpackRequest);

  const testLdapSettings: ApiHook['settings']['testLdapSettings'] = () =>
    client.get('/ldap/test').then(unpackRequest);

  const getGroupsInfo: ApiHook['groups']['getGroupsInfo'] = () =>
    client.get('/group-info').then(unpackRequest);

  const deleteGroup: ApiHook['groups']['deleteGroup'] = (group) =>
    client.delete(`/group/${group}`);

  const addUsersToGroups: ApiHook['groups']['addUsersToGroups'] = (data) =>
    client.post('/groups-assign', data).then(unpackRequest);

  const fetchOpenIdProvider: ApiHook['settings']['fetchOpenIdProviders'] = async () =>
    client.get<OpenIdProvider>(`/openid/provider`).then((res) => res.data);

  const addOpenIdProvider: ApiHook['settings']['addOpenIdProvider'] = async (data) =>
    client.post(`/openid/provider`, data).then(unpackRequest);

  const deleteOpenIdProvider: ApiHook['settings']['deleteOpenIdProvider'] = async (
    name,
  ) => client.delete(`/openid/provider/${name}`).then(unpackRequest);

  const editOpenIdProvider: ApiHook['settings']['editOpenIdProvider'] = async (data) =>
    client.put(`/openid/provider/${data.name}`, data).then(unpackRequest);

  const openIdCallback: ApiHook['auth']['openid']['callback'] = (data) =>
    client.post('/openid/callback', data).then((response) => {
      if (response.status === 200) {
        return response.data;
      }
      if (response.status === 201) {
        return {
          mfa: response.data as MFALoginResponse,
        };
      }
      return {};
    });

  useEffect(() => {
    client.interceptors.response.use(
      (res) => {
        // API sometimes returns null in optional fields.
        if (res.data) {
          res.data = removeNulls(res.data);
        }
        return res;
      },
      (err: ApiError) => {
        if (props?.notifyError) {
          const responseMessage = err.response?.data.msg || err.response?.data.message;
          if (responseMessage) {
            toaster.error(responseMessage.trim());
          } else {
            toaster.error(LL.messages.error());
          }
        }
        return Promise.reject(err);
      },
    );
    return () => {
      client.interceptors.response.clear();
    };
  }, [LL.messages, client.interceptors.response, toaster, props?.notifyError]);

  return {
    getAppInfo,
    changePasswordSelf,
    getEnterpriseStatus,
    oAuth: {
      consent: oAuthConsent,
    },
    groups: {
      deleteGroup,
      getGroupsInfo,
      getGroups,
      createGroup,
      editGroup,
      addUsersToGroups,
    },
    user: {
      getMe,
      addUser,
      getUser,
      getUsers,
      editUser,
      deleteUser,
      usernameAvailable,
      changePassword,
      resetPassword,
      walletChallenge,
      setWallet,
      deleteWallet,
      addToGroup,
      removeFromGroup,
      startEnrollment,
      startDesktopActivation,
      getAuthenticationKeysInfo: getAuthenticationKeysInfo,
      addAuthenticationKey,
      deleteAuthenticationKey,
      renameAuthenticationKey,
      deleteYubiKey,
      renameYubikey,
    },
    device: {
      addDevice: addDevice,
      getDevice: fetchDevice,
      getDevices: fetchDevices,
      getUserDevices: fetchUserDevices,
      editDevice: modifyDevice,
      deleteDevice,
      downloadDeviceConfig,
    },
    network: {
      addNetwork,
      importNetwork,
      mapUserDevices: mapUserDevices,
      getNetwork: fetchNetwork,
      getNetworks: fetchNetworks,
      editNetwork: modifyNetwork,
      deleteNetwork,
      getUsersStats,
      getNetworkToken,
      getNetworkStats,
      getGatewaysStatus,
      deleteGateway,
    },
    auth: {
      login,
      logout,
      openid: {
        getOpenIdInfo: getOpenidInfo,
        callback: openIdCallback,
      },
      mfa: {
        disable: mfaDisable,
        enable: mfaEnable,
        recovery: recovery,
        webauthn: {
          register: {
            start: mfaWebauthnRegisterStart,
            finish: mfaWebauthnRegisterFinish,
          },
          start: mfaWebauthnStart,
          finish: mfaWebautnFinish,
          deleteKey: mfaWebauthnDeleteKey,
        },
        totp: {
          init: mfaTOTPInit,
          enable: mfaTOTPEnable,
          disable: mfaTOTPDisable,
          verify: mfaTOTPVerify,
        },
        email: {
          register: {
            start: mfaEmailMFAInit,
            finish: mfaEmailMFAEnable,
          },
          disable: mfaEmailMFADisable,
          sendCode: mfaEmailMFASendCode,
          verify: mfaEmailMFAVerify,
        },
        web3: {
          start: mfaWeb3Start,
          finish: mfaWeb3Finish,
          updateWalletMFA: editWalletMFA,
        },
      },
    },
    provisioning: {
      provisionYubiKey: provisionYubiKey,
      getWorkers: getWorkers,
      getJobStatus: getJobStatus,
      getWorkerToken: getWorkerToken,
      deleteWorker,
    },
    webhook: {
      getWebhooks: getWebhooks,
      deleteWebhook: deleteWebhook,
      addWebhook: addWebhook,
      changeWebhookState: changeWebhookState,
      editWebhook: editWebhook,
    },
    openid: {
      getOpenidClients: getOpenidClients,
      addOpenidClient: addOpenidClient,
      getOpenidClient: getOpenidClient,
      editOpenidClient: editOpenidClient,
      deleteOpenidClient: deleteOpenidClient,
      changeOpenidClientState: changeOpenidClientState,
      verifyOpenidClient: verifyOpenidClient,
      getUserClients: getUserClients,
      removeUserClient: removeUserClient,
    },
    settings: {
      getSettings: getSettings,
      editSettings: editSettings,
      setDefaultBranding: setDefaultBranding,
      patchSettings,
      getEssentialSettings,
      getEnterpriseSettings,
      patchEnterpriseSettings,
      testLdapSettings,
      fetchOpenIdProviders: fetchOpenIdProvider,
      addOpenIdProvider,
      deleteOpenIdProvider,
      editOpenIdProvider,
    },
    support: {
      downloadSupportData,
      downloadLogs,
    },
    mail: {
      sendTestMail: sendTestMail,
      sendSupportMail: sendSupportMail,
    },
  };
};

export default useApi;
