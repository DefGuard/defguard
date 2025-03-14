export type FiltersDialogFilterItem = {
  label: string;
  value: number;
  searchValues: string[];
};

export type FilterDialogFilter = {
  label: string;
  items: FiltersDialogFilterItem[];
};
