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
import api from '../../shared/api/api';
import { type AclAlias, AclProtocolName, type AclRule } from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
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
  disableBlockedModal?: boolean;
};

export const AliasTable = ({ data: rowData, rules, disableBlockedModal }: Props) => {
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
      invalidate: ['acl', 'alias'],
    },
  });

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Alias name',
        meta: {
          flex: true,
        },
        enableSorting: true,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('addresses', {
        header: 'IP4/6 CIDR range address',
        enableSorting: false,
        size: 430,
        cell: (info) => {
          const value = info.getValue();
          return <TableValuesListCell values={value.split(',')} />;
        },
      }),
      columnHelper.accessor('ports', {
        header: 'Ports',
        enableSorting: false,
        size: 145,
        cell: (info) => <TableValuesListCell values={info.getValue().split(',')} />,
      }),
      columnHelper.accessor('protocols', {
        header: 'Protocols',
        enableSorting: false,
        size: 163,
        cell: (info) => {
          const value = info.getValue();
          if (value.length === 0) {
            return (
              <TableCell>
                <span>All protocols</span>
              </TableCell>
            );
          }
          const nameMap = value.map((protocol) => AclProtocolName[protocol]);
          return <TableValuesListCell values={nameMap} />;
        },
      }),
      columnHelper.accessor('rules', {
        header: 'Used in rules',
        size: 400,
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
                          (ruleId) => rulesById?.[ruleId]?.name ?? `Rule ${ruleId}`,
                        );
                        openModal(ModalName.DeleteAliasDestinationBlocked, {
                          title: 'Deletion blocked',
                          description:
                            'This alias is currently in use by the following rule(s) and cannot be deleted. To proceed, remove it from these rules first:',
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
              text: 'Deploy',
              icon: 'deploy',
              onClick: () => {
                if (licenseInfo === undefined) return;
                licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
                  applyAliases([row.id]);
                });
              },
            });
          }
          return (
            <TableCell>
              <IconButtonMenu
                icon="menu"
                menuItems={menuItems}
                disabled={isLicenseFetching}
              />
            </TableCell>
          );
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
