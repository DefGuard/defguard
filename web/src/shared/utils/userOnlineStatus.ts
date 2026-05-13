import type { Device, User } from '../api/types';

type OnlineDevice = Pick<Device, 'networks'>;
type OnlineUser = Pick<User, 'devices'>;

export const isDeviceOnline = (device: OnlineDevice): boolean =>
  device.networks.some((network) => network.is_active);

export const isUserOnline = (user: OnlineUser): boolean =>
  user.devices.some(isDeviceOnline);
