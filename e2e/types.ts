export type ApiDevice = {
  id?: string;
  name: string;
  wireguard_id: string;
  wireguard_pubKey: string;
};

export type ApiUser = {
  username: string;
  first_name: string;
  last_name: string;
  email: string;
  phone: string;
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
  redirectURL: string;
  scopes: OpenIdScope[];
};

export type OpenIdScope = 'openid' | 'profile' | 'email' | 'phone';
