import axios from 'axios';
import qs from 'qs';

function trimStrings<T>(value: T): T {
  if (typeof value === 'string') {
    return value.trim() as unknown as T;
  }
  if (Array.isArray(value)) {
    return value.map(trimStrings) as unknown as T;
  }
  if (value !== null && typeof value === 'object') {
    const trimmed = {} as T;
    for (const key in value) {
      if (Object.hasOwn(value as object, key)) {
        trimmed[key] = trimStrings(
          (value as Record<string, unknown>)[key],
        ) as T[typeof key];
      }
    }
    return trimmed;
  }
  return value;
}

const TRIM_METHODS = ['post', 'patch', 'put'];

export const client = axios.create({
  baseURL: '/api/v1',
  headers: { 'Content-Type': 'application/json' },
  paramsSerializer: {
    serialize: (params) =>
      qs.stringify(params, {
        arrayFormat: 'repeat',
      }),
  },
});

client.interceptors.request.use((config) => {
  if (
    config.method &&
    TRIM_METHODS.includes(config.method.toLowerCase()) &&
    config.data !== null &&
    config.data !== undefined &&
    typeof config.data === 'object'
  ) {
    config.data = trimStrings(config.data);
  }
  return config;
});
