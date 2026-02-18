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
import { type AclAlias, AclProtocolName } from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { getLicenseInfoQueryOptions } from '../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../shared/utils/license';

type RowData = AclAlias;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  data: RowData[];
};

export const AliasTable = ({ data: rowData }: Props) => {
  const navigate = useNavigate();

  const { data: licenseInfo, isFetching: isLicenseFetching } = useQuery(
    getLicenseInfoQueryOptions,
  );

  const { data: rules } = useQuery({
    queryFn: api.acl.rule.getRules,
    queryKey: ['acl', 'rule'],
    select: (resp) => resp.data,
  });

  const { mutate: deleteAlias } = useMutation({
    mutationFn: api.acl.alias.deleteAlias,
    meta: {
      invalidate: ['acl'],
    },
  });

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
          const value = info.getValue();
          let inRules: string[] = [];
          if (isPresent(rules)) {
            inRules = rules
              .filter((rule) => value.includes(rule.id))
              .map((rule) => rule.name);
          }
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
                  onClick: () => {
                    if (licenseInfo === undefined) return;
                    licenseActionCheck(canUseBusinessFeature(licenseInfo), () => {
                      deleteAlias(row.id);
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
    [rules, applyAliases, deleteAlias, navigate, isLicenseFetching, licenseInfo],
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
