import { ReactNode } from 'react';

export type ManagementPageProps = {
  children: ReactNode;
  title: string;
  search?: ManagementPageSearch;
  actions?: ReactNode;
  itemsCount?: ManagementPageItemsCount;
};

export type ManagementPageItemsCount = {
  itemsCount?: number;
  label: string;
};

export type ManagementPageSearch = {
  onSearch: (searchValue: string) => void;
  placeholder?: string;
  loading?: boolean;
};
