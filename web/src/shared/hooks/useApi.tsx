import axios, { AxiosResponse } from 'axios';

import {
  AddOpenidClientRequest,
  AddUserRequest,
  AddWalletRequest,
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
  RemoveUserClientRequest,
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

interface HookProps {
  baseURL?: string;
}

const envBaseUrl = import.meta.env.API_BASE_URL;

const client = axios.create({
  baseURL: envBaseUrl && String(envBaseUrl).length > 0 ? envBaseUrl : '/api/v1',
});

client.defaults.headers.common['Content-Type'] = 'application/json';

const unpackRequest = <T,>(res: AxiosResponse<T>): T => res.data;

const useApi = (props?: HookProps): ApiHook => {
  if (props) {
    const { baseURL } = props;
    if (baseURL && baseURL.length) {
      client.defaults.baseURL = baseURL;
    }
  }

  client.interceptors.response.use((res) => {
    // API sometimes returns null in optional fields.
    if (res.data) {
      res.data = removeNulls(res.data);
    }
    return res;
  });

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

  const usernameAvailable = (username: string) =>
    client.post('/user/available', { username });

  const getWorkers = () => client.get('/worker').then((res) => res.data);

  const provisionYubiKey = (data: WorkerJobRequest) =>
    client.post<WorkerJobResponse>(`/worker/job`, data).then((response) => response.data);

  const getJobStatus = (id?: number) =>
    client.get<WorkerJobStatus>(`/worker/${id}`).then((res) => res.data);

  const changePassword = ({ username, ...rest }: ChangePasswordRequest) =>
    client.put<EmptyApiResponse>(`/user/${username}/password`, rest);

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

  const mfaEnable = () => client.put('/auth/mfa').then(unpackRequest);

  const recovery: ApiHook['auth']['mfa']['recovery'] = (data) =>
    client.post('/auth/recovery', data).then(unpackRequest);

  const getAppInfo: ApiHook['getAppInfo'] = () => client.get('/info').then(unpackRequest);

  const setDefaultBranding: ApiHook['settings']['setDefaultBranding'] = (id: string) =>
    client.get(`/settings/${id}`).then(unpackRequest);

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

  return {
    getAppInfo,
    changePasswordSelf,
    oAuth: {
      consent: oAuthConsent,
    },
    groups: {
      getGroups,
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
      walletChallenge,
      setWallet,
      deleteWallet,
      addToGroup,
      removeFromGroup,
      startEnrollment,
      startDesktopActivation,
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
