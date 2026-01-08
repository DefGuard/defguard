import type {
  AvailableLocationIpResponse,
  Device,
  GatewayTokenResponse,
  GroupInfo,
  NetworkDevice,
  NetworkLocation,
  OpenIdClient,
  StartEnrollmentResponse,
  User,
  Webhook,
} from '../../api/types';

export interface OpenEditDeviceModal {
  device: Device;
  reservedNames: string[];
  username: string;
}

export interface OpenAuthKeyRenameModal {
  id: number;
  name: string;
  username: string;
}

export interface OpenAddApiTokenModal {
  username: string;
}

export interface OpenRenameApiTokenModal {
  id: number;
  name: string;
  username: string;
}

export interface OpenDeleteApiTokenModal {
  id: number;
  username: string;
}

export interface OpenCEGroupModal {
  groupInfo?: GroupInfo;
  reservedNames: string[];
  users: User[];
}

export interface OpenEditUserModal {
  user: User;
  reservedUsernames: string[];
  reservedEmails: string[];
}

export interface OpenCEOpenIdClientModal {
  openIdClient?: OpenIdClient;
  reservedNames: string[];
}

export interface OpenCEWebhookModal {
  webhook?: Webhook;
}

export interface OpenAssignUsersToGroupsModal {
  users: number[];
  groups: GroupInfo[];
}

export interface OpenAddNetworkDeviceModal {
  locations: NetworkLocation[];
  availableIps: AvailableLocationIpResponse;
  reservedNames: string[];
}
export interface OpenEditNetworkDeviceModal {
  device: NetworkDevice;
  reservedNames: string[];
}
export interface OpenNetworkDeviceConfigModal {
  device: NetworkDevice;
  config: string;
}
export interface OpenNetworkDeviceTokenModal {
  device: NetworkDevice;
  enrollment: StartEnrollmentResponse;
}

export interface OpenDisplayListModal {
  title?: string;
  data: string[];
}

export interface OpenGatewaySetupModal {
  networkId: number;
  data: GatewayTokenResponse;
}

export interface OpenLicenseModal {
  license?: string | null;
}
