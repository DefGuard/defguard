import axios from 'axios';

export const getApiErrorMessage = (error: unknown): string | null => {
  if (axios.isAxiosError(error)) {
    const message = error.response?.data?.msg;
    return typeof message === 'string' ? message : null;
  }

  return null;
};
