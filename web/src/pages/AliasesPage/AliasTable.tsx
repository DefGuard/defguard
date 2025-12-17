import { useQuery } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo } from 'react';
import api from '../../shared/api/api';
import { type AclAlias, AclProtocolName } from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';

type RowData = AclAlias;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  data: RowData[];
};

export const AliasTable = ({ data: rowData }: Props) => {
  const { data: rules } = useQuery({
    queryFn: api.acl.rule.getRules,
    queryKey: ['acl', 'rule'],
    select: (resp) => resp.data,
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
      columnHelper.accessor('destination', {
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
    ],
    [rules],
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
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return <TableBody table={table} />;
};
