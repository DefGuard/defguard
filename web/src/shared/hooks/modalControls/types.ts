import type { Device, GroupInfo, OpenIdClient, User, Webhook } from '../../api/types';

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
