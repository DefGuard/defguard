import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import type { AclListTabValue } from '../../shared/aclTabs';
import api from '../../shared/api/api';
import { type AclAlias, AclProtocolName, type AclRule } from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { getLicenseInfoQueryOptions } from '../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../shared/utils/license';
import { resourceById } from '../../shared/utils/resourceById';

type RowData = AclAlias;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  data: RowData[];
  rules: AclRule[];
  tab: AclListTabValue;
  disableBlockedModal?: boolean;
};

export const AliasTable = ({ data: rowData, rules, tab, disableBlockedModal }: Props) => {
  const navigate = useNavigate();

  const { data: licenseInfo, isFetching: isLicenseFetching } = useQuery(
    getLicenseInfoQueryOptions,
  );

  const rulesById = useMemo(() => resourceById(rules), [rules]);
  const rulesByAliasId = useMemo(() => {
    if (!rules) return {} as Record<number, string[]>;
    const map: Record<number, string[]> = {};
    rules.forEach((rule) => {
      rule.aliases.forEach((aliasId) => {
        if (!map[aliasId]) {
          map[aliasId] = [];
        }
        map[aliasId].push(rule.name);
      });
    });
    return map;
  }, [rules]);

  const { mutate: applyAliases } = useMutation({
    mutationFn: api.acl.alias.applyAliases,
    meta: {
      invalidate: ['acl'],
    },
  });

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.acl_alias_col_name(),
        enableSorting: true,
        sortingFn: 'text',
        minSize: 300,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('addresses', {
        header: m.acl_alias_col_cidr_range_address(),
        enableSorting: false,
        size: 430,
        minSize: 300,
        cell: (info) => {
          const value = info.getValue();
          return <TableValuesListCell values={value.split(',')} />;
        },
      }),
      columnHelper.accessor('ports', {
        header: m.acl_col_ports(),
        enableSorting: false,
        size: 145,
        minSize: 145,
        cell: (info) => <TableValuesListCell values={info.getValue().split(',')} />,
      }),
      columnHelper.accessor('protocols', {
        header: m.acl_col_protocols(),
        enableSorting: false,
        size: 163,
        minSize: 163,
        cell: (info) => {
          const value = info.getValue();
          if (value.length === 0) {
            return (
              <TableCell>
                <span>{m.acl_protocols_all()}</span>
              </TableCell>
            );
          }
          const nameMap = value.map((protocol) => AclProtocolName[protocol]);
          return <TableValuesListCell values={nameMap} />;
        },
      }),
      columnHelper.accessor('rules', {
        header: m.acl_col_used_in_rules(),
        size: 400,
        minSize: 300,
        enableSorting: false,
        cell: (info) => {
          const row = info.row.original;
          const aliasId = row.parent_id ?? row.id;
          const inRules = rulesByAliasId[aliasId] ?? [];
          return <TableValuesListCell values={inRules} />;
        },
      }),
      columnHelper.display({
        id: 'edit',
        enableSorting: false,
        size: tableEditColumnSize,
        cell: (info) => {
          const row = info.row.original;
          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                {
                  text: m.controls_edit(),
                  icon: 'edit',
                  onClick: () => {
                    if (licenseInfo === undefined) return;
                    licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
                      navigate({
                        to: '/acl/edit-alias',
                        search: {
                          alias: row.id,
                          tab,
                        },
                      });
                    });
                  },
                },
                {
                  text: m.controls_delete(),
                  icon: 'delete',
                  variant: 'danger',
                  disabled: disableBlockedModal && row.rules.length > 0,
                  onClick: () => {
                    if (licenseInfo === undefined) return;
                    licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
                      if (row.rules.length > 0) {
                        const ruleNames = row.rules.map(
                          (ruleId) =>
                            rulesById?.[ruleId]?.name ??
                            m.acl_rule_fallback_name({ id: ruleId }),
                        );
                        openModal(ModalName.DeleteAliasDestinationBlocked, {
                          title: m.modal_delete_acl_blocked_title(),
                          description: m.modal_delete_acl_alias_blocked_body(),
                          rules: ruleNames,
                        });
                        return;
                      }
                      openModal(ModalName.DeleteAliasDestinationConfirm, {
                        title: m.modal_delete_acl_alias_title(),
                        description: m.modal_delete_acl_alias_body(),
                        target: {
                          kind: 'alias',
                          id: row.id,
                        },
                      });
                    });
                  },
                },
              ],
            },
          ];
          if (row.state === 'Modified') {
            menuItems[0].items.splice(1, 0, {
              text: m.controls_deploy(),
              icon: 'deploy',
              onClick: () => {
                if (licenseInfo === undefined) return;
                licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
                  applyAliases([row.id]);
                });
              },
            });
          }
          return <TableEditCell menuItems={menuItems} disabled={isLicenseFetching} />;
        },
      }),
    ],
    [
      rulesById,
      rulesByAliasId,
      applyAliases,
      disableBlockedModal,
      navigate,
      isLicenseFetching,
      licenseInfo,
      tab,
    ],
  );

  const table = useReactTable({
    initialState: {
      sorting: [
        {
          id: 'name',
          desc: false,
        },
      ],
    },
    data: rowData,
    columns,
    enableRowSelection: false,
    enableExpanding: false,
    enableSorting: true,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return <TableBody table={table} />;
};
