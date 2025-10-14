import { get } from './api-client';
import type { User } from './types';

const api = {
  user: {
    getMe: get<User>('/me'),
  },
} as const;

export default api;
