export type DeviceNetworkInfo = {
  device_wireguard_ips: string[];
  is_active: boolean;
  network_gateway_ip: string;
  network_id: number;
  network_name: string;
  last_connected_at?: string;
  last_connected_ip?: string;
};

export type ApiDevice = {
  id: number;
  user_id: number;
  name: string;
  wireguard_pubkey: string;
  created: string;
  networks: DeviceNetworkInfo[];
};

export enum AuthenticationKeyType {
  SSH = 'ssh',
  GPG = 'gpg',
}

export type ApiUserAuthKey = {
  id: number;
  name?: string;
  key_type: AuthenticationKeyType;
  key: string;
  yubikey_serial?: string;
  yubikey_id?: number;
  yubikey_name?: string;
};

export type ApiUser = {
  username: string;
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
};

export type ApiUserProfile = {
  user: ApiUser;
  devices: ApiDevice[];
};

export type User = {
  username: string;
  firstName: string;
  lastName: string;
  password: string;
  mail: string;
  phone: string;
};

export type OpenIdClient = {
  name: string;
  clientID?: string;
  clientSecret?: string;
  redirectURL: string[];
  scopes: OpenIdScope[];
};

export type NetworkForm = {
  name: string;
  address: string;
  endpoint: string;
  port: string;
  allowed_ips?: string;
  dns?: string;
};

export type DeviceForm = {
  name: string;
  pubKey?: string;
};

export type NetworkDeviceForm = {
  name: string;
  pubKey?: string;
  description?: string;
};

export type EditNetworkDeviceForm = {
  name?: string;
  ip?: string;
  description?: string;
};

export type OpenIdScope = 'openid' | 'profile' | 'email' | 'phone';
