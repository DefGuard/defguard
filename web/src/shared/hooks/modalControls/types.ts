import type { Device, User } from '../../api/types';

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

export interface OpenAddGroupModal {
  reservedNames: string[];
  users: User[];
}
