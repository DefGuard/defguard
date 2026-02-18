import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { flat } from 'radashi';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type AclAlias,
  type AclRule,
  AclStatus,
  type AclStatusValue,
  type GroupInfo,
  type LicenseInfo,
  type NetworkDevice,
  type NetworkLocation,
  type ResourceById,
  type User,
} from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { IconButtonMenu } from '../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { canUseBusinessFeature, licenseActionCheck } from '../../shared/utils/license';

const displayUser = (user?: User): string => {
  if (!isPresent(user)) return '~';

  if (user.first_name || user.last_name) {
    return `${user.first_name} ${user.last_name}`;
  }
  return user.username;
};

type RowData = AclRule;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  license: LicenseInfo | null;
  aliases: ResourceById<AclAlias>;
  groups: ResourceById<GroupInfo>;
  users: ResourceById<User>;
  devices: ResourceById<NetworkDevice>;
  locations: ResourceById<NetworkLocation>;
  data: AclRule[];
  title: string;
  buttonProps: ButtonProps;
  enableSearch?: boolean;
};

export const RulesTable = ({
  title,
  buttonProps,
  enableSearch,
  aliases,
  devices,
  groups,
  users,
  locations,
  data,
  license,
}: Props) => {
  const navigate = useNavigate();

  const { mutate: deleteRule } = useMutation({
    mutationFn: api.acl.rule.deleteRule,
    meta: {
      invalidate: ['acl'],
    },
  });

  const [search, setSearch] = useState('');

  const renderPermissionCell = useCallback(
    (
      permission: 'deny' | 'allow',
      permissionUsers: boolean,
      permissionGroup: boolean,
      permissionDevice: boolean,
      includedUsers: number[],
      includedGroups: number[],
      includedDevices: number[],
    ) => {
      if (permissionDevice && permissionGroup && permissionUsers) {
        return (
          <TableCell>
            {permission === 'allow' && (
              <Badge
                variant={BadgeVariant.Success}
                icon="check-filled"
                text="All allowed"
              />
            )}
            {permission === 'deny' && (
              <Badge
                variant={BadgeVariant.Warning}
                icon="status-important"
                text="All denied"
              />
            )}
          </TableCell>
        );
      }
      const displayValues: string[][] = [];
      if (!permissionGroup) {
        displayValues.push(includedGroups.map((groupId) => groups[groupId]?.name ?? ''));
      }
      if (!permissionUsers) {
        displayValues.push(includedUsers.map((userId) => displayUser(users[userId])));
      }
      if (!permissionDevice) {
        displayValues.push(
          includedDevices.map((deviceId) => devices[deviceId]?.name ?? ''),
        );
      }
      const display = flat(displayValues).filter((value) => !value.length);

      return <TableValuesListCell values={display} />;
    },
    [users, devices, groups],
  );

  const renderStatusCell = useCallback(
    (ruleState: AclStatusValue, isEnabled: boolean) => {
      // handle applied rules first
      if (ruleState === AclStatus.Applied) {
        return (
          <TableCell>
            {isEnabled ? (
              <Badge variant={BadgeVariant.Success} text="Active" />
            ) : (
              <Badge variant={BadgeVariant.Critical} text="Disabled" />
            )}
          </TableCell>
        );
      } else if (ruleState === AclStatus.Deleted) {
        return (
          <TableCell>
            <Badge variant={BadgeVariant.Critical} text="Deleted" />
          </TableCell>
        );
      } else {
        // handle remaining pending states
        return (
          <TableCell>
            <Badge variant={BadgeVariant.Warning} text={ruleState} />
          </TableCell>
        );
      }
    },
    [],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: 'Rule name',
        minSize: 210,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'destination',
        header: 'Destination',
        minSize: 350,
        cell: (info) => {
          const row = info.row.original;
          return (
            <TableCell>
              <span>{row.addresses}</span>
              {row.aliases.map((aliasId) => {
                const alias = aliases[aliasId];
                if (!alias) return null;
                return <Badge variant="neutral" text={alias.name} key={aliasId} />;
              })}
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'permissions',
        header: 'Permissions',
        minSize: 220,
        cell: (info) => {
          const row = info.row.original;
          return renderPermissionCell(
            'allow',
            row.allow_all_users,
            row.allowed_groups.length === 0,
            row.allow_all_network_devices,
            row.allowed_users,
            row.allowed_groups,
            row.allowed_network_devices,
          );
        },
      }),
      columnHelper.display({
        id: 'restrictions',
        header: 'Restrictions',
        minSize: 220,
        cell: (info) => {
          const row = info.row.original;
          return renderPermissionCell(
            'deny',
            row.deny_all_users,
            row.denied_groups.length === 0,
            row.deny_all_network_devices,
            row.denied_users,
            row.denied_groups,
            row.denied_network_devices,
          );
        },
      }),
      columnHelper.display({
        id: 'locations',
        header: 'Locations',
        minSize: 220,
        cell: (info) => {
          const row = info.row.original;
          if (row.all_locations) {
            return (
              <TableCell>
                <Badge
                  variant={BadgeVariant.Success}
                  text="All locations"
                  icon="check-filled"
                />
              </TableCell>
            );
          }
          const locationNames = row.locations
            .map((locationId) => locations[locationId]?.name ?? '')
            .filter((name) => name.length);

          return <TableValuesListCell values={locationNames} />;
        },
      }),
      columnHelper.display({
        id: 'status',
        header: 'Status',
        cell: (info) => {
          const row = info.row.original;
          return renderStatusCell(row.state, row.enabled);
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
                  icon: 'edit',
                  text: m.controls_edit(),
                  onClick: () => {
                    licenseActionCheck(canUseBusinessFeature(license), () => {
                      navigate({
                        to: '/acl/edit-rule',
                        search: {
                          rule: row.id,
                        },
                      });
                    });
                  },
                },
              ],
            },
            {
              items: [
                {
                  icon: 'delete',
                  variant: 'danger',
                  text: m.controls_delete(),
                  onClick: () => {
                    licenseActionCheck(canUseBusinessFeature(license), () => {
                      deleteRule(row.id);
                    });
                  },
                },
              ],
            },
          ];
          return (
            <TableCell>
              <IconButtonMenu icon="menu" menuItems={menuItems} />
            </TableCell>
          );
        },
      }),
    ],
    [
      aliases,
      renderPermissionCell,
      deleteRule,
      locations,
      navigate,
      renderStatusCell,
      license,
    ],
  );

  const visibleRules = useMemo(() => {
    let res = data;
    if (search.length) {
      res = res.filter((rule) => rule.name.toLowerCase().includes(search.toLowerCase()));
    }
    return res;
  }, [search, data]);

  const table = useReactTable({
    columns,
    data: visibleRules,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
  });

  if (data.length === 0) return null;

  return (
    <>
      <TableTop text={title}>
        {enableSearch && (
          <Search placeholder={m.controls_search()} value={search} onChange={setSearch} />
        )}
        <Button {...buttonProps} />
      </TableTop>
      {visibleRules.length > 0 && <TableBody table={table} />}
      {visibleRules.length === 0 && (
        <EmptyStateFlexible
          icon="search"
          title={m.search_empty_common_title()}
          subtitle={m.search_empty_common_subtitle()}
        />
      )}
    </>
  );
};
