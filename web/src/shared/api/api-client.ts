import axios from 'axios';
import qs from 'qs';

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
