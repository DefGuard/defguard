import axios, { AxiosPromise, AxiosResponse } from 'axios';
import { toast } from 'react-toastify';

import ToastContent, { ToastType } from '../components/Toasts/ToastContent';
import {
  AddDeviceRequest,
  AddOpenidClientRequest,
  AddUserRequest,
  AddWalletRequest,
  AddWebhookRequest,
  ApiHook,
  AuthorizedClient,
  ChangeOpenidClientStateRequest,
  ChangePasswordRequest,
  changeWebhookStateRequest,
  Device,
  EditOpenidClientRequest,
  EditWebhookRequest,
  EmptyApiResponse,
  GetNetworkStatsRequest,
  GroupsResponse,
  License,
  LoginData,
  Network,
  NetworkToken,
  NetworkUserStats,
  OpenidClient,
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
import { useAppStore } from './store/useAppStore';

interface HookProps {
  baseURL?: string;
}

const client = axios.create({
  baseURL: '/api/v1',
});
client.defaults.headers.common['Content-Type'] = 'application/json';

const errorToast = (message: string) =>
  toast(<ToastContent type={ToastType.ERROR} message={message} />);

const unpackRequest = <T,>(res: AxiosResponse<T>): T => res.data;

const useApi = (props?: HookProps): ApiHook => {
  const [backendVersion, setAppStore] = useAppStore((state) => [
    state.backendVersion,
    state.setAppStore,
  ]);

  if (props) {
    const { baseURL } = props;
    if (baseURL && baseURL.length) {
      client.defaults.baseURL = baseURL;
    }
  }

  client.interceptors.response.use(
    (res) => {
      if (res && res.headers) {
        const version = res.headers['x-defguard-version'] as string | undefined;
        if (version && version.length) {
          if (backendVersion !== version) {
            setAppStore({ backendVersion: version });
          }
        }
      }
      return res;
    },
    (error) => {
      if (Number(error.code) > 401 && Number(error.code) < 500) {
        const message = error.response?.data.message;
        if (message) {
          errorToast(String(message));
        }
      }
      return Promise.reject(error);
    }
  );

  const addUser = async (data: AddUserRequest) => {
    return client
      .post<EmptyApiResponse>(`/user/`, data)
      .then((res) => res.data);
  };

  const getMe = () => client.get<User>(`/me`).then((res) => res.data);

  const getUser: ApiHook['user']['getUser'] = async (username: string) =>
    client.get<User>(`/user/${username}`).then((res) => res.data);

  const fetchUsers = async () =>
    client.get<User[]>(`/user/`).then((res) => res.data);

  const modifyUser = async ({ username, data }: UserEditRequest) =>
    client.put<User>(`/user/${username}`, data).then(unpackRequest);

  const deleteUser = async (user: User) =>
    client
      .delete<EmptyApiResponse>(`/user/${user.username}`)
      .then((res) => res.data);

  const fetchDevices = async () =>
    client.get<Device[]>(`/device`).then((res) => res.data);

  const fetchDevice = async (id: string) =>
    client.get<Device>(`/device/${id}`).then((res) => res.data);

  const downloadDeviceConfig = async (id: string) =>
    client.get<string>(`/device/${id}/config`).then((res) => res.data);

  const modifyDevice = async (device: Device) =>
    client.put<Device>(`/device/${device.id}`, device).then((res) => res.data);

  const deleteDevice = async (device: Device) =>
    client.delete<EmptyApiResponse>(`/device/${device.id}`);

  const addDevice = async ({ username, ...rest }: AddDeviceRequest) =>
    client.post<Device>(`/device/${username}`, rest).then((res) => res.data);

  const fetchUserDevices = async (username: string) =>
    client.get<Device[]>(`/device/user/${username}`).then((res) => res.data);

  const fetchNetworks = async () =>
    client.get<Network[]>(`/network`).then((res) => res.data);

  const fetchNetwork = async (id: string) =>
    client.get<Network>(`/network/${id}`).then((res) => res.data);

  const modifyNetwork = async (network: Network) =>
    client
      .put<Network>(`/network/${network.id}`, network)
      .then((res) => res.data);

  const deleteNetwork = async (network: Network) =>
    client.delete<EmptyApiResponse>(`/network/${network.id}`);

  const addNetwork = async (network: Network) =>
    client.post<EmptyApiResponse>(`/network/`, network).then((res) => res.data);

  const login = (data: LoginData): AxiosPromise => client.post('/auth', data);

  const logout = () =>
    client.post<EmptyApiResponse>('/auth/logout').then(unpackRequest);

  const usernameAvailable = (username: string) =>
    client.post('/user/available', { username });

  const getWorkers = () => client.get('/worker/').then((res) => res.data);

  const provisionYubiKey = (data: WorkerJobRequest) =>
    client
      .post<WorkerJobResponse>(`/worker/job`, data)
      .then((response) => response.data);

  const getJobStatus = (id?: number) =>
    client.get<WorkerJobStatus>(`/worker/${id}`).then((res) => res.data);

  const changePassword = ({ username, ...rest }: ChangePasswordRequest) =>
    client.put<EmptyApiResponse>(`/user/${username}/password`, rest);

  const walletChallenge = ({
    username,
    address,
    name,
    chainId,
  }: WalletChallengeRequest) =>
    client
      .get<WalletChallenge>(
        `/user/${username}/challenge?address=${address}&name=${name}&chain_id=${chainId}`
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

  const getGroups = () =>
    client.get<GroupsResponse>('/group/').then(unpackRequest);

  const addToGroup = ({ group, ...rest }: UserGroupRequest) =>
    client.post<EmptyApiResponse>(`/group/${group}`, rest);

  const removeFromGroup = ({ group, username }: UserGroupRequest) =>
    client.delete(`/group/${group}/user/${username}`);

  const deleteWorker = (id: string) =>
    client.delete<EmptyApiResponse>(`/worker/${id}`).then((res) => res.data);

  const getWebhooks = () => client.get('/webhook/').then((res) => res.data);

  const deleteWebhook = (id: string) =>
    client.delete<EmptyApiResponse>(`/webhook/${id}`).then((res) => res.data);

  const changeWebhookState = ({ id, ...rest }: changeWebhookStateRequest) =>
    client.post<EmptyApiResponse>(`/webhook/${id}`, rest);

  const addWebhook = async (data: AddWebhookRequest) => {
    return client.post<EmptyApiResponse>('/webhook/', data);
  };
  const editWebhook = async ({ id, ...rest }: EditWebhookRequest) => {
    return client.put<EmptyApiResponse>(`/webhook/${id}`, rest);
  };
  const getOpenidClients = () => client.get('/openid/').then((res) => res.data);

  const getOpenidClient = async (id: string) =>
    client.get<OpenidClient>(`/openid/${id}`).then((res) => res.data);

  const addOpenidClient = async (data: AddOpenidClientRequest) => {
    return client.post<EmptyApiResponse>('/openid/', data);
  };
  const editOpenidClient = async ({ id, ...rest }: EditOpenidClientRequest) => {
    return client.put<EmptyApiResponse>(`/openid/${id}`, rest);
  };
  const changeOpenidClientState = async ({
    id,
    ...rest
  }: ChangeOpenidClientStateRequest) => {
    return client.post<EmptyApiResponse>(`/openid/${id}`, rest);
  };
  const deleteOpenidClient = async (id: string) =>
    client.delete<EmptyApiResponse>(`/openid/${id}`).then((res) => res.data);

  const verifyOpenidClient = async (data: VerifyOpenidClientRequest) =>
    client.post('openid/verify', data);

  const getUserClients = async (username: string) =>
    client
      .get<AuthorizedClient[]>(`/openid/apps/${username}`)
      .then((res) => res.data);

  const removeUserClient = async (id: string) =>
    client
      .delete<EmptyApiResponse>(`/openid/apps/${id}`)
      .then((res) => res.data);

  const oAuthConsent = (params: unknown) =>
    client
      .post('/oauth/authorize', null, {
        params: params,
      })
      .then((res) => res.data);

  const getUsersStats = (data?: GetNetworkStatsRequest) =>
    client
      .get<NetworkUserStats[]>('/network/stats/users', {
        params: {
          ...data,
        },
      })
      .then(unpackRequest);

  const getNetworkToken = (id: string) =>
    client.get<NetworkToken>(`/network/token/${id}`).then(unpackRequest);

  const getNetworkStats = (data?: GetNetworkStatsRequest) =>
    client
      .get<WireguardNetworkStats>('/network/stats', {
        params: {
          ...data,
        },
      })
      .then(unpackRequest);

  const getWorkerToken = () =>
    client.get<WorkerToken>('/worker/token').then(unpackRequest);

  const getLicense = () =>
    client.get<License>('/license/').then((res) => res.data);

  return {
    oAuth: {
      consent: oAuthConsent,
    },
    groups: {
      getGroups,
    },
    user: {
      getMe,
      addUser,
      getUser: getUser,
      getUsers: fetchUsers,
      editUser: modifyUser,
      deleteUser,
      usernameAvailable,
      changePassword,
      walletChallenge,
      setWallet,
      deleteWallet,
      addToGroup,
      removeFromGroup,
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
      addNetwork: addNetwork,
      getNetwork: fetchNetwork,
      getNetworks: fetchNetworks,
      editNetwork: modifyNetwork,
      deleteNetwork,
      getUsersStats,
      getNetworkToken,
      getNetworkStats,
    },
    auth: {
      login,
      logout,
    },
    license: {
      getLicense: getLicense,
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
  };
};

export default useApi;
