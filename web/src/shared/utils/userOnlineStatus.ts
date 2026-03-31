import type { Device, User } from '../api/types';

export const isDeviceOnline = (device: Device): boolean =>
  device.networks.some((network) => network.is_active);

export const isUserOnline = (user: User): boolean => user.devices.some(isDeviceOnline);
