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
