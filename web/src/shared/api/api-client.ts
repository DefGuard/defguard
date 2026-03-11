import axios from 'axios';
import qs from 'qs';
import { router } from '../../app/router';
import { useAuth } from '../hooks/useAuth';

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

client.interceptors.response.use(
  (response) => response,
  async (error) => {
    if (axios.isAxiosError(error) && error.response?.status === 403) {
      const username = useAuth.getState().user?.username;

      if (username) {
        const currentPath = router.state.location.pathname;
        const profilePath = `/user/${username}`;

        if (currentPath !== profilePath) {
          await router.navigate({
            to: '/user/$username',
            params: { username },
            replace: true,
          });
        }
      }
    }

    return Promise.reject(error);
  },
);
