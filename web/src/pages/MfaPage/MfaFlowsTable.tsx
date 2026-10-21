import { useNavigate } from '@tanstack/react-router';
import {
  type ColumnFiltersState,
  createColumnHelper,
  type FilterFn,
  getCoreRowModel,
  getFilteredRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import type { MfaFlowListItemResponse } from '../../shared/api/types';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import { Icon } from '../../shared/defguard-ui/components/Icon/Icon';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import type { TableFilterMessages } from '../../shared/defguard-ui/components/table/types';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeVariable } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { closeModal, openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import {
  getDeleteMfaFlowModalData,
  getMfaFlowDeleteBlockingLocations,
  getMfaFlowDeleteConflictLocations,
  openMfaFlowDeleteBlockedModal,
} from './mfaFlows';

type Props = {
  flows: MfaFlowListItemResponse[];
  addButtonProps: ButtonProps;
};

const columnHelper = createColumnHelper<MfaFlowListItemResponse>();

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
  const locationOptions = useMemo(
    (): SelectionOption<number>[] =>
      [
        ...new Map(
          flows.flatMap((flow) => flow.locations).map((loc) => [loc.id, loc.name]),
        ),
      ]
        .sort(([, left], [, right]) => left.localeCompare(right))
        .map(([id, label]) => ({ id, label })),
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
            {isPresent(info.row.original.unavailable_reason) && (
              <TooltipProvider>
                <TooltipTrigger>
                  <Icon
                    icon={IconKind.WarningFilled}
                    staticColor={ThemeVariable.FgAttention}
                  />
                </TooltipTrigger>
                <TooltipContent>
                  <p>{m.mfa_flow_unavailable_warning()}</p>
                </TooltipContent>
              </TooltipProvider>
            )}
          </TableCell>
        ),
      }),
      columnHelper.accessor('step_count', {
        header: m.mfa_flows_table_steps(),
        size: 140,
        enableColumnFilter: true,
        filterFn: filterByStepCount,
        meta: { filterOptions: stepCountOptions },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor((flow) => flow.locations.map((location) => location.id), {
        id: 'locations',
        header: m.mfa_flows_table_locations(),
        minSize: 350,
        enableColumnFilter: true,
        filterFn: 'arrIncludesSome',
        meta: { flex: true, filterOptions: locationOptions },
        cell: (info) => {
          const { locations } = info.row.original;
          return (
            <TableCell>
              <span>
                {locations.length > 0
                  ? locations.map((location) => location.name).join(', ')
                  : '-'}
              </span>
            </TableCell>
          );
        },
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
                    const blockingLocations = getMfaFlowDeleteBlockingLocations(flow);
                    if (blockingLocations.length > 0) {
                      openMfaFlowDeleteBlockedModal(blockingLocations);
                      return;
                    }
                    openModal(ModalName.ConfirmAction, {
                      ...getDeleteMfaFlowModalData(flow),
                      onSuccess: () => Snackbar.default(m.mfa_flow_deleted()),
                      onError: (_message, _code, error) => {
                        // The list can be stale, so the server refusal opens the same modal.
                        const conflictLocations =
                          getMfaFlowDeleteConflictLocations(error);
                        if (isPresent(conflictLocations)) {
                          closeModal(ModalName.ConfirmAction);
                          openMfaFlowDeleteBlockedModal(conflictLocations);
                          return;
                        }
                        Snackbar.error(m.mfa_flow_delete_failed());
                      },
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
    [navigate, stepCountOptions, locationOptions],
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
