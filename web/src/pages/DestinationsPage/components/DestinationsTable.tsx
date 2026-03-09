import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import {
  type AclDestination,
  AclProtocolName,
  type AclRule,
} from '../../../shared/api/types';
import { TableValuesListCell } from '../../../shared/components/TableValuesListCell/TableValuesListCell';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconButtonMenu } from '../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { getLicenseInfoQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { resourceById } from '../../../shared/utils/resourceById';

type Props = {
  title: string;
  destinations: AclDestination[];
  rules: AclRule[];
  primaryProps: ButtonProps;
  search?: boolean;
  disableBlockedModal?: boolean;
};

type RowData = AclDestination;

const columnHelper = createColumnHelper<RowData>();

export const DestinationsTable = ({
  primaryProps,
  destinations,
  rules,
  title,
  search,
  disableBlockedModal,
}: Props) => {
  const rulesById = useMemo(() => resourceById(rules), [rules]);
  const rulesByDestinationId = useMemo(() => {
    if (!rules) return {} as Record<number, string[]>;
    const map: Record<number, string[]> = {};
    rules.forEach((rule) => {
      rule.destinations.forEach((destinationId) => {
        if (!map[destinationId]) {
          map[destinationId] = [];
        }
        map[destinationId].push(rule.name);
      });
    });
    return map;
  }, [rules]);
  const [searchValue, setSearchValue] = useState<string>('');
  const navigate = useNavigate();

  const { data: licenseInfo, isFetching: licenseFetching } = useQuery(
    getLicenseInfoQueryOptions,
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Destination name',
        minSize: 210,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'destinations',
        header: 'IP4/6 CIDR range addresses',
        minSize: 300,
        cell: (info) => {
          const row = info.row.original;
          if (row.any_address) {
            return (
              <TableCell>
                <span>{`Any`}</span>
              </TableCell>
            );
          }
          return <TableValuesListCell values={row.addresses.split(',')} />;
        },
      }),
      columnHelper.display({
        id: 'ports',
        header: 'Ports',
        minSize: 230,
        cell: (info) => {
          const row = info.row.original;
          if (row.any_port) {
            return (
              <TableCell>
                <span>{`Any port`}</span>
              </TableCell>
            );
          }
          return <TableValuesListCell values={row.ports.split(',')} />;
        },
      }),
      columnHelper.display({
        id: 'protocols',
        header: 'Protocols',
        minSize: 230,
        cell: (info) => {
          const row = info.row.original;
          if (row.any_protocol) {
            return (
              <TableCell>
                <span>{`Any protocol`}</span>
              </TableCell>
            );
          }
          const display = row.protocols.map((protocol) => AclProtocolName[protocol]);
          return <TableValuesListCell values={display} />;
        },
      }),
      columnHelper.display({
        id: 'rules',
        header: 'Used in rules',
        minSize: 500,
        cell: (info) => {
          if (!rulesById) return null;
          const row = info.row.original;
          const destinationId = row.parent_id ?? row.id;
          const display = rulesByDestinationId[destinationId] ?? [];
          return <TableValuesListCell values={display} />;
        },
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        enableResizing: false,
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
                        to: '/acl/edit-destination',
                        search: {
                          destination: row.id,
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
                        const ruleNames = rulesById
                          ? row.rules.map(
                              (ruleId) => rulesById[ruleId]?.name ?? `Rule ${ruleId}`,
                            )
                          : row.rules.map((ruleId) => `Rule ${ruleId}`);
                        openModal(ModalName.DeleteAliasDestinationBlocked, {
                          title: 'Deletion blocked',
                          description:
                            'This destination is currently in use by the following rule(s) and cannot be deleted. To proceed, remove it from these rules first:',
                          rules: ruleNames,
                        });
                        return;
                      }
                      openModal(ModalName.DeleteAliasDestinationConfirm, {
                        title: m.modal_delete_acl_destination_title(),
                        description: m.modal_delete_acl_destination_body(),
                        target: {
                          kind: 'destination',
                          id: row.id,
                        },
                      });
                    });
                  },
                },
              ],
            },
          ];
          return (
            <TableCell>
              <IconButtonMenu
                icon="menu"
                menuItems={menuItems}
                disabled={licenseFetching}
              />
            </TableCell>
          );
        },
      }),
    ],
    [
      navigate,
      rulesById,
      rulesByDestinationId,
      licenseFetching,
      licenseInfo,
      disableBlockedModal,
    ],
  );

  const transformedData = useMemo(() => {
    if (!searchValue.length) {
      return destinations;
    }

    const normalizedSearchValue = searchValue.toLowerCase();

    return destinations.filter((destination) => {
      if (destination.name.toLowerCase().includes(normalizedSearchValue)) {
        return true;
      }

      const destinationId = destination.parent_id ?? destination.id;
      const ruleNames = rulesByDestinationId[destinationId] ?? [];

      return ruleNames.some((ruleName) =>
        ruleName.toLowerCase().includes(normalizedSearchValue),
      );
    });
  }, [searchValue, destinations, rulesByDestinationId]);

  const table = useReactTable({
    columns,
    data: transformedData,
    enableExpanding: false,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <>
      <TableTop text={title}>
        {search && (
          <Search
            onChange={setSearchValue}
            placeholder={m.controls_search()}
            value={searchValue}
          />
        )}
        <Button {...primaryProps} />
      </TableTop>
      {transformedData.length > 0 && <TableBody table={table} />}
      {transformedData.length === 0 && (
        <EmptyStateFlexible
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
    </>
  );
};
