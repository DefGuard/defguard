import { get, post } from './api-client';
import type { LoginRequest, LoginResponse, User } from './types';

const api = {
  user: {
    getMe: get<User>('/me'),
  },
  auth: {
    login: post<LoginRequest, LoginResponse>('/auth'),
  },
} as const;

export default api;
