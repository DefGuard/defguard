import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { MfaFlowListItemResponse } from '../../shared/api/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';

/** Data and actions required by the MFA flow table. */
type Props = {
  flows: MfaFlowListItemResponse[];
  addButtonProps: ButtonProps;
};

const columnHelper = createColumnHelper<MfaFlowListItemResponse>();

/** Displays configured MFA flows and their available row actions. */
export const MfaFlowsTable = ({ flows, addButtonProps }: Props) => {
  const navigate = useNavigate();
  const [search, setSearch] = useState('');
  const columns = useMemo(
    () => [
      columnHelper.accessor('title', {
        header: m.mfa_flows_table_title(),
        minSize: 306,
        meta: { flex: true },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('step_count', {
        header: m.mfa_flows_table_steps(),
        size: 140,
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
                      onError: () => Snackbar.error(m.mfa_flow_delete_failed()),
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
    [navigate],
  );
  const table = useReactTable({
    columns,
    data: flows,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <>
      <TableTop text={m.mfa_flows_table_heading()}>
        <Search placeholder={m.controls_search()} value={search} onChange={setSearch} />
        <Button {...addButtonProps} />
      </TableTop>
      <TableBody table={table} />
    </>
  );
};
