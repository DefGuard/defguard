import type { ColumnFiltersState } from '@tanstack/react-table';

export type ColumnFilterParams = Record<string, Array<string | number>>;

/** Maps non-empty multi-select column filters to API query params named after column IDs. */
export const mapColumnFiltersToApiParams = (
  columnFilters: ColumnFiltersState,
): ColumnFilterParams => {
  const result: ColumnFilterParams = {};
  for (const filter of columnFilters) {
    if (Array.isArray(filter.value) && filter.value.length > 0) {
      result[filter.id] = filter.value.filter(
        (value): value is string | number =>
          typeof value === 'string' || typeof value === 'number',
      );
    }
  }
  return result;
};
