import { useApiStore } from './api/store';

export const useApi = () => {
  const endpoints = useApiStore((s) => s.endpoints);

  if (!endpoints) {
    throw Error('Used API hook before it was initialized.');
  }

  return endpoints;
};

export default useApi;
