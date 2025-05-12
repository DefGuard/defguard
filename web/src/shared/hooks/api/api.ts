import { Axios, AxiosResponse } from 'axios';

import {
  AddDeviceResponse,
  AddOpenidClientRequest,
  AddUserRequest,
  Api,
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
  LoginResponse,
  MFALoginResponse,
  Network,
  NetworkToken,
  OpenidClient,
  OpenIdInfo,
  Provisioner,
  RemoveUserClientRequest,
  ResetPasswordRequest,
  Settings,
  StartEnrollmentRequest,
  StartEnrollmentResponse,
  User,
  UserEditRequest,
  UserGroupRequest,
  UserProfile,
  VerifyOpenidClientRequest,
  WireguardNetworkStats,
  WorkerJobRequest,
  WorkerJobResponse,
  WorkerJobStatus,
  WorkerToken,
} from '../../types';
import { UpdateInfo } from '../store/useUpdatesStore';

const unpackRequest = <T>(res: AxiosResponse<T>): T => res.data;

export const buildApi = (client: Axios): Api => {
  const addUser = async (data: AddUserRequest) => {
    return client.post<User>(`/user`, data).then(unpackRequest);
  };

  const getMe = () => client.get<User>(`/me`).then(unpackRequest);

  const getUser: Api['user']['getUser'] = async (username) =>
    client.get<UserProfile>(`/user/${username}`).then(unpackRequest);

  const editUser = async ({ username, data }: UserEditRequest) =>
    client.put<User>(`/user/${username}`, data).then(unpackRequest);

  const deleteUser = async (user: User) =>
    client.delete<EmptyApiResponse>(`/user/${user.username}`).then(unpackRequest);

  const fetchDevices = async () => client.get<Device[]>(`/device`).then(unpackRequest);

  const fetchDevice = async (id: string) =>
    client.get<Device>(`/device/${id}`).then(unpackRequest);

  const getUsers = () => client.get('/user').then(unpackRequest);

  const downloadDeviceConfig: Api['device']['downloadDeviceConfig'] = async (data) =>
    client
      .get<string>(`/network/${data.network_id}/device/${data.device_id}/config`)
      .then(unpackRequest);

  const modifyDevice = async (device: Device) =>
    client.put<Device>(`/device/${device.id}`, device).then(unpackRequest);

  const deleteDevice = async (device: Device) =>
    client.delete<EmptyApiResponse>(`/device/${device.id}`);

  const addDevice: Api['device']['addDevice'] = async ({ username, ...rest }) =>
    client.post<AddDeviceResponse>(`/device/${username}`, rest).then(unpackRequest);

  const fetchUserDevices = async (username: string) =>
    client.get<Device[]>(`/device/user/${username}`).then(unpackRequest);

  const fetchNetworks = async () => client.get<Network[]>(`/network`).then(unpackRequest);

  const fetchNetwork = async (id: number) =>
    client.get<Network>(`/network/${id}`).then(unpackRequest);

  // For now there is only one network
  const modifyNetwork: Api['network']['editNetwork'] = async (data) =>
    client.put<Network>(`/network/${data.id}`, data.network).then(unpackRequest);

  const deleteNetwork: Api['network']['deleteNetwork'] = async (id) =>
    client.delete<EmptyApiResponse>(`/network/${id}`);

  const addNetwork: Api['network']['addNetwork'] = (network) =>
    client.post(`/network`, network).then(unpackRequest);

  const importNetwork: Api['network']['importNetwork'] = (network) =>
    client.post(`/network/import`, network).then(unpackRequest);

  const mapUserDevices: Api['network']['mapUserDevices'] = (data) =>
    client
      .post(`/network/${data.networkId}/devices`, { devices: data.devices })
      .then(unpackRequest);

  const login: Api['auth']['login'] = (data: LoginData) =>
    client.post('/auth', data).then((response) => {
      if (response.status === 200) {
        return response.data as LoginResponse;
      }
      if (response.status === 201) {
        return {
          mfa: response.data as MFALoginResponse,
        };
      }
      return {};
    });

  const logout = () => client.post<EmptyApiResponse>('/auth/logout').then(unpackRequest);

  const getOpenidInfo: Api['auth']['openid']['getOpenIdInfo'] = () =>
    client.get(`/openid/auth_info`).then(unpackRequest);

  const usernameAvailable = (username: string) =>
    client.post('/user/available', { username });

  const getWorkers: Api['provisioning']['getWorkers'] = () =>
    client.get<Provisioner[]>('/worker').then(unpackRequest);

  const provisionYubiKey = (data: WorkerJobRequest) =>
    client.post<WorkerJobResponse>(`/worker/job`, data).then((response) => response.data);

  const getJobStatus = (id?: number) =>
    client.get<WorkerJobStatus>(`/worker/${id}`).then(unpackRequest);

  const changePassword = ({ username, ...rest }: ChangePasswordRequest) =>
    client.put<EmptyApiResponse>(`/user/${username}/password`, rest);

  const resetPassword = ({ username }: ResetPasswordRequest) =>
    client.post<EmptyApiResponse>(`/user/${username}/reset_password`);

  const startEnrollment = ({ username, ...rest }: StartEnrollmentRequest) =>
    client
      .post<StartEnrollmentResponse>(`/user/${username}/start_enrollment`, rest)
      .then((response) => response.data);

  const getGroups = () => client.get<GroupsResponse>('/group').then(unpackRequest);

  const addToGroup = ({ group, ...rest }: UserGroupRequest) =>
    client.post<EmptyApiResponse>(`/group/${group}`, rest);

  const removeFromGroup = ({ group, username }: UserGroupRequest) =>
    client.delete(`/group/${group}/user/${username}`);

  const createGroup: Api['groups']['createGroup'] = (data) =>
    client.post(`/group`, data).then(unpackRequest);

  const editGroup: Api['groups']['editGroup'] = ({ originalName, ...rest }) =>
    client.put(`/group/${originalName}`, rest).then(unpackRequest);

  const deleteWorker = (id: string) =>
    client.delete<EmptyApiResponse>(`/worker/${id}`).then(unpackRequest);

  const getWebhooks = () => client.get('/webhook').then(unpackRequest);

  const deleteWebhook = (id: string) =>
    client.delete<EmptyApiResponse>(`/webhook/${id}`).then(unpackRequest);

  const changeWebhookState = ({ id, ...rest }: changeWebhookStateRequest) =>
    client.post<EmptyApiResponse>(`/webhook/${id}`, rest);

  const addWebhook: Api['webhook']['addWebhook'] = async (data) => {
    return client.post<EmptyApiResponse>('/webhook', data);
  };
  const editWebhook: Api['webhook']['editWebhook'] = async ({ id, ...rest }) => {
    return client.put<EmptyApiResponse>(`/webhook/${id}`, rest);
  };
  const getOpenidClients = () => client.get('/oauth').then(unpackRequest);

  const getOpenidClient = async (client_id: string) =>
    client.get<OpenidClient>(`/oauth/${client_id}`).then(unpackRequest);

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
    client.delete<EmptyApiResponse>(`/oauth/${id}`).then(unpackRequest);

  const verifyOpenidClient = async (data: VerifyOpenidClientRequest) =>
    client.post('openid/verify', data);

  const getUserClients = async (username: string) =>
    client.get<AuthorizedClient[]>(`/oauth/apps/${username}`).then(unpackRequest);

  const removeUserClient = async (data: RemoveUserClientRequest) =>
    client
      .delete<EmptyApiResponse>(`/user/${data.username}/oauth_app/${data.client_id}`)
      .then(unpackRequest);

  const oAuthConsent = (params: unknown) =>
    client
      .post('/oauth/authorize', null, {
        params: params,
      })
      .then(unpackRequest);

  const getOverviewStats: Api['network']['getOverviewStats'] = (
    data: GetNetworkStatsRequest,
  ) =>
    client
      .get(`/network/${data.id}/stats/users`, {
        params: {
          ...data,
        },
      })
      .then(unpackRequest);

  const getNetworkToken: Api['network']['getNetworkToken'] = (networkId) =>
    client.get<NetworkToken>(`/network/${networkId}/token`).then(unpackRequest);

  const getNetworkStats: Api['network']['getNetworkStats'] = (data) =>
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

  const mfaWebauthnRegisterStart: Api['auth']['mfa']['webauthn']['register']['start'] =
    () => client.post('/auth/webauthn/init').then(unpackRequest);

  const mfaWebauthnRegisterFinish: Api['auth']['mfa']['webauthn']['register']['finish'] =
    (data) => client.post('/auth/webauthn/finish', data).then(unpackRequest);

  const mfaWebauthnStart = () => client.post('/auth/webauthn/start').then(unpackRequest);

  const mfaWebautnFinish: Api['auth']['mfa']['webauthn']['finish'] = (data) =>
    client.post('/auth/webauthn', data).then(unpackRequest);

  const mfaTOTPInit = () => client.post('/auth/totp/init').then(unpackRequest);

  const mfaTOTPEnable: Api['auth']['mfa']['totp']['enable'] = (data) =>
    client.post('/auth/totp', data).then(unpackRequest);

  const mfaTOTPDisable = () => client.delete('/auth/totp').then(unpackRequest);

  const mfaTOTPVerify: Api['auth']['mfa']['totp']['verify'] = (data) =>
    client.post('/auth/totp/verify', data).then(unpackRequest);

  const mfaEmailMFAInit: Api['auth']['mfa']['email']['register']['start'] = () =>
    client.post('/auth/email/init').then(unpackRequest);

  const mfaEmailMFAEnable: Api['auth']['mfa']['email']['register']['finish'] = (data) =>
    client.post('/auth/email', data).then(unpackRequest);

  const mfaEmailMFADisable = () => client.delete('/auth/email').then(unpackRequest);

  const mfaEmailMFASendCode: Api['auth']['mfa']['email']['sendCode'] = () =>
    client.get('/auth/email').then(unpackRequest);

  const mfaEmailMFAVerify: Api['auth']['mfa']['email']['verify'] = (data) =>
    client.post('/auth/email/verify', data).then(unpackRequest);

  const mfaWebauthnDeleteKey: Api['auth']['mfa']['webauthn']['deleteKey'] = ({
    keyId,
    username,
  }) => client.delete(`/user/${username}/security_key/${keyId}`);

  const getSettings = () => client.get('/settings').then(unpackRequest);

  const editSettings = (settings: Settings) =>
    client.put('/settings', settings).then(unpackRequest);

  const getEnterpriseInfo = () => client.get('/enterprise_info').then(unpackRequest);

  const mfaEnable = () => client.put('/auth/mfa').then(unpackRequest);

  const recovery: Api['auth']['mfa']['recovery'] = (data) =>
    client.post('/auth/recovery', data).then(unpackRequest);

  const getAppInfo: Api['getAppInfo'] = () => client.get('/info').then(unpackRequest);

  const setDefaultBranding: Api['settings']['setDefaultBranding'] = (id: string) =>
    client.put(`/settings/${id}`).then(unpackRequest);

  const downloadSupportData: Api['support']['downloadSupportData'] = async () =>
    client.get<unknown>(`/support/configuration`).then(unpackRequest);

  const downloadLogs: Api['support']['downloadLogs'] = async () =>
    client.get<string>(`/support/logs`).then(unpackRequest);

  const getGatewaysStatus: Api['network']['getGatewaysStatus'] = (networkId) =>
    client.get(`/network/${networkId}/gateways`).then(unpackRequest);

  const deleteGateway: Api['network']['deleteGateway'] = (data) =>
    client.delete(`/network/${data.networkId}/gateways/${data.gatewayId}`);

  const changePasswordSelf: Api['changePasswordSelf'] = (data) =>
    client.put('/user/change_password', data).then(unpackRequest);

  const sendTestMail: Api['mail']['sendTestMail'] = (data) =>
    client.post('/mail/test', data).then(unpackRequest);

  const sendSupportMail: Api['mail']['sendSupportMail'] = () =>
    client.post('/mail/support', {}).then(unpackRequest);

  const startDesktopActivation: Api['user']['startDesktopActivation'] = (data) =>
    client.post(`/user/${data.username}/start_desktop`, data).then(unpackRequest);

  const getAuthenticationKeysInfo: Api['user']['getAuthenticationKeysInfo'] = (data) =>
    client.get(`/user/${data.username}/auth_key`).then(unpackRequest);

  const addAuthenticationKey: Api['user']['addAuthenticationKey'] = (data) =>
    client.post(`/user/${data.username}/auth_key`, data).then(unpackRequest);

  const renameAuthenticationKey: Api['user']['renameAuthenticationKey'] = (data) =>
    client
      .post(`/user/${data.username}/auth_key/${data.id}/rename`, {
        name: data.name,
      })
      .then(unpackRequest);

  const deleteAuthenticationKey: Api['user']['deleteAuthenticationKey'] = (data) =>
    client.delete(`/user/${data.username}/auth_key/${data.id}`).then(unpackRequest);

  const renameYubikey: Api['user']['renameYubikey'] = (data) =>
    client
      .post(`/user/${data.username}/yubikey/${data.id}/rename`, {
        name: data.name,
      })
      .then(unpackRequest);

  const deleteYubiKey: Api['user']['deleteYubiKey'] = (data) =>
    client.delete(`/user/${data.username}/yubikey/${data.id}`).then(unpackRequest);

  const getApiTokensInfo: Api['user']['getApiTokensInfo'] = (data) =>
    client.get(`/user/${data.username}/api_token`).then(unpackRequest);

  const addApiToken: Api['user']['addApiToken'] = (data) =>
    client.post(`/user/${data.username}/api_token`, data).then(unpackRequest);

  const renameApiToken: Api['user']['renameApiToken'] = (data) =>
    client
      .post(`/user/${data.username}/api_token/${data.id}/rename`, {
        name: data.name,
      })
      .then(unpackRequest);

  const deleteApiToken: Api['user']['deleteApiToken'] = (data) =>
    client.delete(`/user/${data.username}/api_token/${data.id}`).then(unpackRequest);

  const patchSettings: Api['settings']['patchSettings'] = (data) =>
    client.patch('/settings', data).then(unpackRequest);

  const getEssentialSettings: Api['settings']['getEssentialSettings'] = () =>
    client.get('/settings_essentials').then(unpackRequest);

  const getEnterpriseSettings: Api['settings']['getEnterpriseSettings'] = () =>
    client.get('/settings_enterprise').then(unpackRequest);

  const patchEnterpriseSettings: Api['settings']['patchEnterpriseSettings'] = (data) =>
    client.patch('/settings_enterprise', data).then(unpackRequest);

  const testLdapSettings: Api['settings']['testLdapSettings'] = () =>
    client.get('/ldap/test').then(unpackRequest);

  const getGroupsInfo: Api['groups']['getGroupsInfo'] = () =>
    client.get('/group-info').then(unpackRequest);

  const deleteGroup: Api['groups']['deleteGroup'] = (group) =>
    client.delete(`/group/${group}`);

  const addUsersToGroups: Api['groups']['addUsersToGroups'] = (data) =>
    client.post('/groups-assign', data).then(unpackRequest);

  const fetchOpenIdProvider: Api['settings']['fetchOpenIdProviders'] = () =>
    client.get<OpenIdInfo>(`/openid/provider`).then(unpackRequest);

  const addOpenIdProvider: Api['settings']['addOpenIdProvider'] = (data) =>
    client.post(`/openid/provider`, data).then(unpackRequest);

  const deleteOpenIdProvider: Api['settings']['deleteOpenIdProvider'] = (name) =>
    client.delete(`/openid/provider/${name}`).then(unpackRequest);

  const editOpenIdProvider: Api['settings']['editOpenIdProvider'] = (data) =>
    client.put(`/openid/provider/${data.name}`, data).then(unpackRequest);

  const openIdCallback: Api['auth']['openid']['callback'] = (data) =>
    client.post('/openid/callback', data).then((response) => {
      if (response.status === 200) {
        return response.data as LoginResponse;
      }
      if (response.status === 201) {
        const mfa = response.data as MFALoginResponse;
        return {
          mfa,
        } as LoginResponse;
      }
      return {};
    });

  const getNewVersion: Api['getNewVersion'] = () =>
    client.get('/updates').then((res) => {
      if (res.status === 204) {
        return null;
      }
      return res.data as UpdateInfo;
    });

  const testDirsync: Api['settings']['testDirsync'] = () =>
    client.get('/test_directory_sync').then(unpackRequest);

  const createStandaloneDevice: Api['standaloneDevice']['createManualDevice'] = (data) =>
    client.post('/device/network', data).then(unpackRequest);

  const deleteStandaloneDevice: Api['standaloneDevice']['deleteDevice'] = (deviceId) =>
    client.delete(`/device/network/${deviceId}`);
  const editStandaloneDevice: Api['standaloneDevice']['editDevice'] = ({ id, ...data }) =>
    client.put(`/device/network/${id}`, data).then(unpackRequest);

  const getStandaloneDevice: Api['standaloneDevice']['getDevice'] = (deviceId) =>
    client.get(`/device/network/${deviceId}`).then(unpackRequest);

  const getAvailableLocationIp: Api['standaloneDevice']['getAvailableIp'] = (data) =>
    client.get(`/device/network/ip/${data.locationId}`).then(unpackRequest);

  const validateLocationIp: Api['standaloneDevice']['validateLocationIp'] = ({
    location,
    ...rest
  }) => client.post(`/device/network/ip/${location}`, rest).then(unpackRequest);

  const getStandaloneDevicesList: Api['standaloneDevice']['getDevicesList'] = () =>
    client.get('/device/network').then(unpackRequest);

  const createStandaloneCliDevice: Api['standaloneDevice']['createCliDevice'] = (data) =>
    client.post('/device/network/start_cli', data).then(unpackRequest);

  const getStandaloneDeviceConfig: Api['standaloneDevice']['getDeviceConfig'] = (id) =>
    client.get(`/device/network/${id}/config`).then(unpackRequest);

  const generateStandaloneDeviceAuthToken: Api['standaloneDevice']['generateAuthToken'] =
    (id) => client.post(`/device/network/start_cli/${id}`).then(unpackRequest);

  const createAclRule: Api['acl']['rules']['createRule'] = (data) =>
    client.post('/acl/rule', data).then(unpackRequest);

  const editAclRule: Api['acl']['rules']['editRule'] = ({ id, ...rest }) =>
    client.put(`/acl/rule/${id}`, rest).then(unpackRequest);

  const getAclRules: Api['acl']['rules']['getRules'] = () =>
    client.get('/acl/rule').then(unpackRequest);

  const getAclRule: Api['acl']['rules']['getRule'] = (id: number) =>
    client.get(`/acl/rule/${id}`).then(unpackRequest);

  const deleteAclRule: Api['acl']['rules']['deleteRule'] = (id) =>
    client.delete(`/acl/rule/${id}`).then(unpackRequest);

  const getAliases: Api['acl']['aliases']['getAliases'] = () =>
    client.get(`/acl/alias`).then(unpackRequest);

  const getAlias: Api['acl']['aliases']['getAlias'] = (id) =>
    client.get(`/acl/alias/${id}`).then(unpackRequest);

  const createAlias: Api['acl']['aliases']['createAlias'] = (data) =>
    client.post(`/acl/alias`, data).then(unpackRequest);

  const editAlias: Api['acl']['aliases']['editAlias'] = (data) =>
    client.put(`/acl/alias/${data.id}`, data).then(unpackRequest);

  const deleteAlias: Api['acl']['aliases']['deleteAlias'] = (id) =>
    client.delete(`/acl/alias/${id}`).then(unpackRequest);

  const applyAclRules: Api['acl']['rules']['applyRules'] = (rules) =>
    client
      .put('/acl/rule/apply', {
        rules: rules,
      })
      .then(unpackRequest);

  const applyAclAliases: Api['acl']['aliases']['applyAliases'] = (aliases) =>
    client
      .put(`/acl/alias/apply`, {
        aliases: aliases,
      })
      .then(unpackRequest);

  const getAuditLog: Api['auditLog']['getAuditLog'] = (params) =>
    client
      .get(`/audit_log`, {
        params,
      })
      .then(unpackRequest);

  return {
    getAppInfo,
    getNewVersion,
    changePasswordSelf,
    getEnterpriseInfo,
    auditLog: {
      getAuditLog,
    },
    acl: {
      aliases: {
        createAlias,
        deleteAlias,
        editAlias,
        getAlias,
        getAliases,
        applyAliases: applyAclAliases,
      },
      rules: {
        createRule: createAclRule,
        getRules: getAclRules,
        getRule: getAclRule,
        editRule: editAclRule,
        deleteRule: deleteAclRule,
        applyRules: applyAclRules,
      },
    },
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
    standaloneDevice: {
      createManualDevice: createStandaloneDevice,
      deleteDevice: deleteStandaloneDevice,
      editDevice: editStandaloneDevice,
      getDevice: getStandaloneDevice,
      getAvailableIp: getAvailableLocationIp,
      validateLocationIp: validateLocationIp,
      getDevicesList: getStandaloneDevicesList,
      createCliDevice: createStandaloneCliDevice,
      getDeviceConfig: getStandaloneDeviceConfig,
      generateAuthToken: generateStandaloneDeviceAuthToken,
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
      addToGroup,
      removeFromGroup,
      startEnrollment,
      startDesktopActivation,
      getAuthenticationKeysInfo,
      addAuthenticationKey,
      deleteAuthenticationKey,
      renameAuthenticationKey,
      deleteYubiKey,
      renameYubikey,
      getApiTokensInfo,
      addApiToken,
      deleteApiToken,
      renameApiToken,
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
      getNetworkToken,
      getNetworkStats,
      getGatewaysStatus,
      deleteGateway,
      getOverviewStats: getOverviewStats,
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
      testDirsync,
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
