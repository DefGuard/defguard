import { useNavigate } from '@tanstack/react-router';
import {
  type ColumnFiltersState,
  createColumnHelper,
  type FilterFn,
  getCoreRowModel,
  getFilteredRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { isAxiosError } from 'axios';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type {
  MfaFlowErrorResponse,
  MfaFlowListItemResponse,
} from '../../shared/api/types';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import type { TableFilterMessages } from '../../shared/defguard-ui/components/table/types';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';

type Props = {
  flows: MfaFlowListItemResponse[];
  addButtonProps: ButtonProps;
};

const columnHelper = createColumnHelper<MfaFlowListItemResponse>();

export const getMfaFlowDeleteErrorMessage = (error: unknown): string => {
  if (!isAxiosError<MfaFlowErrorResponse>(error)) return m.mfa_flow_delete_failed();

  const field = error.response?.data.fields?.[0];
  const locations = field?.locations?.join(', ');
  if (!field || !locations) return m.mfa_flow_delete_failed();

  switch (field.code) {
    case 'location_requires_flow':
      return m.mfa_flow_delete_location_requires_flow({ locations });
    case 'flow_is_default':
      return m.mfa_flow_delete_flow_is_default({ locations });
    default:
      return m.mfa_flow_delete_failed();
  }
};

/** Matches rows whose step count is one of the selected values. */
const filterByStepCount: FilterFn<MfaFlowListItemResponse> = (
  row,
  columnId,
  selectedCounts: number[],
) => selectedCounts.includes(row.getValue<number>(columnId));
filterByStepCount.autoRemove = (value) => !Array.isArray(value) || value.length === 0;

export const MfaFlowsTable = ({ flows, addButtonProps }: Props) => {
  const navigate = useNavigate();
  const [search, setSearch] = useState('');
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const visibleFlows = useMemo(() => {
    const normalizedSearch = search.trim().toLowerCase();
    if (!normalizedSearch) return flows;

    return flows.filter((flow) => flow.title.toLowerCase().includes(normalizedSearch));
  }, [flows, search]);
  const stepCountOptions = useMemo(
    (): SelectionOption<number>[] =>
      [...new Set(flows.map((flow) => flow.step_count))]
        .sort((left, right) => left - right)
        .map((count) => ({ id: count, label: String(count) })),
    [flows],
  );
  const filterMessages: TableFilterMessages = {
    searchPlaceholder: m.controls_search(),
    clearButton: m.controls_reset(),
    applyButton: m.controls_submit(),
    emptyState: m.search_empty_common_title(),
  };
  const columns = useMemo(
    () => [
      columnHelper.accessor('title', {
        header: m.mfa_flows_table_title(),
        minSize: 350,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('step_count', {
        header: m.mfa_flows_table_steps(),
        size: 140,
        enableColumnFilter: true,
        filterFn: filterByStepCount,
        meta: { flex: true, filterOptions: stepCountOptions },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        enableResizing: false,
        cell: (info) => {
          const flow = info.row.original;
          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.controls_edit(),
                  icon: 'edit',
                  onClick: () => {
                    void navigate({
                      to: '/mfa-flow/$id/edit',
                      params: { id: String(flow.id) },
                    });
                  },
                },
              ],
            },
            {
              items: [
                {
                  text: m.controls_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => {
                    openModal(ModalName.ConfirmAction, {
                      title: m.mfa_flow_delete_title(),
                      contentMd: m.mfa_flow_delete_body({ name: flow.title }),
                      actionPromise: () => api.mfaFlow.delete(flow.id),
                      invalidateKeys: [['mfa-flow']],
                      submitProps: { text: m.controls_delete(), variant: 'critical' },
                      onSuccess: () => Snackbar.default(m.mfa_flow_deleted()),
                      onError: (_message, _code, error) =>
                        Snackbar.error(getMfaFlowDeleteErrorMessage(error)),
                    });
                  },
                },
              ],
            },
          ];

          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [navigate, stepCountOptions],
  );
  const table = useReactTable({
    state: { columnFilters },
    meta: { filterMessages },
    columns,
    data: visibleFlows,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    onColumnFiltersChange: setColumnFilters,
    getFilteredRowModel: getFilteredRowModel(),
    getCoreRowModel: getCoreRowModel(),
  });
  const rows = table.getRowModel().rows;
  const hasActiveFilters = search.trim().length > 0 || columnFilters.length > 0;

  return (
    <>
      <TableTop text={m.mfa_flows_table_heading()}>
        <Search placeholder={m.controls_search()} value={search} onChange={setSearch} />
        <Button {...addButtonProps} />
      </TableTop>
      <TableBody table={table} />
      {rows.length === 0 && hasActiveFilters && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
    </>
  );
};
