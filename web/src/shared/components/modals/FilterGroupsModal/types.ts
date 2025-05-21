export type FilterGroupsModalFilterItem = {
  label: string;
  value: number | string;
  searchValues: string[];
};

export type FilterGroupsModalFilter = {
  label: string;
  items: FilterGroupsModalFilterItem[];
  order: number;
  identifier: string;
};
