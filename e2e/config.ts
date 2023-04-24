import { loadEnv } from './utils/loadEnv';

loadEnv();

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
  auth: {
    login: '/auth/login',
  },
  admin: {
    wizard: '/admin/wizard',
    users: '/admin/users',
  },
};

export const defaultUserAdmin = {
  username: 'admin',
  password: 'pass123',
};
