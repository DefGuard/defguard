import { client } from './api-client';
import type { PaginatedResponse } from './types';

export const fetchPage = <T>(
  path: string,
  params?: object,
): Promise<PaginatedResponse<T>> =>
  client
    .get<PaginatedResponse<T>>(path, {
      params,
    })
    .then((resp) => resp.data);

export const fetchAllPages = async <T>(path: string): Promise<T[]> => {
  const data: T[] = [];
  let page: number | null = 1;

  while (page !== null) {
    const response: PaginatedResponse<T> = await fetchPage<T>(path, { page });

    data.push(...response.data);
    page = response.pagination.next_page;
  }

  return data;
};
