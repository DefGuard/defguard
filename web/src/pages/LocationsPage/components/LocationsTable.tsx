import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import type { NetworkLocation } from '../../../shared/api/types';
import { GatewaysStatusBadge } from '../../../shared/components/GatewaysStatusBadge/GatewaysStatusBadge';
import { TableValuesListCell } from '../../../shared/components/TableValuesListCell/TableValuesListCell';
import { Badge } from '../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyState } from '../../../shared/defguard-ui/components/EmptyState/EmptyState';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { tableSortingFns } from '../../../shared/utils/dateSortingFn';

type Props = {
  locations: NetworkLocation[];
};

type RowData = NetworkLocation;

const columnHelper = createColumnHelper<RowData>();

export const LocationsTable = ({ locations }: Props) => {
  const [search, setSearch] = useState('');

  const transformedData = useMemo(() => {
    let res = locations;

    if (search && search.length > 0) {
      res = res.filter((location) =>
        location.name.toLowerCase().includes(search.toLowerCase()),
      );
    }
    return res;
  }, [locations, search]);

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add Location',
      iconLeft: 'add-location',
      onClick: () => {},
    }),
    [],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Location name',
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
      columnHelper.accessor('gateways', {
        header: 'Gateway status',
        size: 175,
        cell: (info) => (
          <TableCell>
            <GatewaysStatusBadge data={info.getValue() ?? []} />
          </TableCell>
        ),
      }),
      columnHelper.accessor('endpoint', {
        header: 'Gateway IP',
        size: 200,
        cell: (info) => {
          return (
            <TableCell>
              <span>{info.getValue()}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('acl_enabled', {
        header: 'Firewall',
        size: 76,
        cell: (info) => (
          <TableCell className="cell-acl-enabled">
            {info.getValue() ? <Icon icon="check-circle" /> : <Icon icon="disabled" />}
          </TableCell>
        ),
      }),
      columnHelper.accessor('address', {
        header: 'VPN network',
        size: 250,
        cell: (info) => <TableValuesListCell values={info.getValue()} />,
      }),
      columnHelper.accessor('location_mfa_mode', {
        header: 'MFA',
        size: 100,
        cell: (info) => {
          switch (info.getValue()) {
            case 'disabled':
              return (
                <TableCell>
                  <Badge text="No MFA" />
                </TableCell>
              );
            case 'external':
              return (
                <TableCell>
                  <Badge icon="external-mfa" text="External" variant="warning" />
                </TableCell>
              );
            case 'internal':
              return (
                <TableCell>
                  <Badge icon="internal-mfa" text="Internal" variant="success" />
                </TableCell>
              );
          }
        },
      }),
      columnHelper.accessor('allowed_groups', {
        header: 'Allowed groups',
        size: 500,
        cell: (info) => {
          const value = info.getValue();
          const len = value?.length ?? 0;
          return (
            <TableCell>
              {len === 0 && <span>All groups allowed</span>}
              {len > 0 && <span>{info.getValue()?.join(', ')}</span>}
            </TableCell>
          );
        },
      }),
    ],
    [],
  );

  const table = useReactTable({
    data: transformedData,
    columns: columns,
    initialState: {
      sorting: [
        {
          id: 'name',
          desc: false,
        },
      ],
    },
    sortingFns: tableSortingFns,
    enableRowSelection: false,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <>
      {locations.length > 0 && (
        <>
          <SizedBox height={ThemeSpacing.Xl3} />
          <TableTop text="Locations management">
            <Search
              placeholder={m.controls_search()}
              onChange={setSearch}
              initialValue={search}
            />
            <Button {...addButtonProps} />
          </TableTop>
          {transformedData.length > 0 && <TableBody table={table} />}
          {transformedData.length === 0 && (
            <EmptyState
              icon="search"
              title={m.search_empty_common_title()}
              subtitle={m.search_empty_common_subtitle()}
            />
          )}
        </>
      )}
      {locations.length === 0 && (
        <EmptyState
          title="No locations found"
          subtitle="Click button below to add one."
          primaryAction={addButtonProps}
        />
      )}
    </>
  );
};
