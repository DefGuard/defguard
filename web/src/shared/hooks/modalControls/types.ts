import type { Device } from '../../api/types';

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
