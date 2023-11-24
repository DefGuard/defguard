import { mergeObjects } from './utils/utils';

type TestsConfig = {
  BASE_URL: string;
  CORE_BASE_URL: string;
  ENROLLMENT_URL: string;
  TEST_TIMEOUT: number;
};

const defaultConfig: TestsConfig = {
  BASE_URL: 'http://localhost:8000',
  CORE_BASE_URL: 'http://localhost:8000/api/v1',
  ENROLLMENT_URL: 'http://localhost:8080',
  TEST_TIMEOUT: 180,
};

const envConfig: Partial<TestsConfig> = {
  BASE_URL: process.env.BASE_URL,
  CORE_BASE_URL: process.env.CORE_BASE_URL,
  ENROLLMENT_URL: process.env.ENROLLMENT_URL,
  TEST_TIMEOUT: process.env.TEST_TIMEOUT
    ? Number.parseInt(process.env.TEST_TIMEOUT)
    : undefined,
};

export const testsConfig: TestsConfig = mergeObjects(envConfig, defaultConfig);

export const routes = {
  base: testsConfig.BASE_URL,
  me: '/me',
  consent: '/consent',
  addDevice: '/add-device',
  auth: {
    login: '/auth/login',
    totp: '/auth/mfa/totp',
    recovery: '/auth/mfa/recovery',
  },
  admin: {
    wizard: '/admin/wizard',
    users: '/admin/users',
    openid: '/admin/openid',
    overview: '/admin/overview',
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
