export type ListHeaderColumnConfig<T> = {
  label: string;
  enabled?: boolean;
  sortKey?: keyof T;
  key?: string;
};
