import type { SortingFn } from '@tanstack/react-table';
import dayjs from 'dayjs';

export const dateSortingFn: SortingFn<unknown> = (row, row2, colId) => {
  console.log({ row, row2, colId });
  const first = dayjs(row.getValue<string>(colId));
  const second = dayjs(row2.getValue<string>(colId));
  return first.valueOf() - second.valueOf();
};

export const tableSortingFns: Record<string, SortingFn<unknown>> = {
  dateIso: dateSortingFn,
} as const;
