import type { Device, UsersListItem } from '../api/types';

export const isDeviceOnline = (device: Device): boolean =>
  device.networks.some((network) => network.is_active);

export const isUserOnline = (user: UsersListItem): boolean =>
  user.devices.some(isDeviceOnline);
