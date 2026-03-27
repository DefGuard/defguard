import '@tanstack/react-table';
import type { RowData } from '@tanstack/react-table';
import type { SelectionOption } from './shared/components/SelectionSection/type';
import type { TableFilterMessages } from './shared/defguard-ui/components/table/types';

declare module '@tanstack/react-table' {
  interface ColumnMeta<TData extends RowData, TValue> {
    flex?: boolean;
    filterOptions?: SelectionOption<unknown>[];
  }

  interface TableMeta<TData extends RowData> {
    filterMessages?: TableFilterMessages;
  }
}
