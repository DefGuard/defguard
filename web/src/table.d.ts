import '@tanstack/react-table';
import type { RowData } from '@tanstack/react-table';
import type { SelectionOption } from './shared/components/SelectionSection/type';

declare module '@tanstack/react-table' {
  interface ColumnMeta<TData extends RowData, TValue> {
    flex?: boolean;
    filterOptions?: SelectionOption<unknown>[];
  }
}
