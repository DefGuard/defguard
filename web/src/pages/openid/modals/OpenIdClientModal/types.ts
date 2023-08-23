export type OpenIdClientFormFields = {
  name: string;
  redirect_uri: {
    url: string;
  }[];
  scope: OpenIdClientScope[];
};

export enum OpenIdClientScope {
  OPENID = 'openid',
  PROFILE = 'profile',
  EMAIL = 'email',
  PHONE = 'phone',
}
