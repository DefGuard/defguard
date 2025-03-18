export type FilterGroupsModalFilterItem = {
  label: string;
  value: number;
  searchValues: string[];
};

export type FilterGroupsModalFilter = {
  label: string;
  items: FilterGroupsModalFilterItem[];
  order: number;
};
