type TestsConfig = {
  BASE_URL: string;
};

const defaultConfig: TestsConfig = {
  BASE_URL: 'http://127.0.0.1:3000',
};

const envConfig: Partial<TestsConfig> = {
  BASE_URL: process.env.BASE_URL,
};

export const testsConfig: TestsConfig = {
  ...defaultConfig,
  ...envConfig,
};

export const routes = {
  base: testsConfig.BASE_URL,
  me: '/me',
  consent: '/consent',
  auth: {
    login: '/auth/login',
    totp: '/auth/mfa/totp',
    recovery: '/auth/mfa/recovery',
  },
  admin: {
    wizard: '/admin/wizard',
    users: '/admin/users',
    openid: '/admin/openid',
  },
  authorize: '/api/v1/oauth/authorize',
};

export const defaultUserAdmin = {
  username: 'admin',
  password: 'pass123',
};

export const testUserTemplate = {
  firstName: 'test first name',
  lastName: 'test last name',
  password: 'defguarD123!',
  mail: 'test@test.com',
  phone: '123456789',
};
