import { client } from './api-client';
import type { User, UserProfileResponse } from './types';

const api = {
  user: {
    getMe: client.get<User>('/me'),
    getUser: (username: string) => client.get<UserProfileResponse>(`/user/${username}`),
    editUser: (data: { username: string; body: User }) =>
      client.put(`/user/${data.username}`, data.body),
  },
} as const;

export default api;
