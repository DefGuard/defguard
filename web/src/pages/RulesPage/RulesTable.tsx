import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { cloneDeep } from 'radashi';
import { useCallback, useMemo, useState } from 'react';
import './RulesTable.scss';
import { m } from '../../paraglide/messages';
import { AclListTab, type AclListTabValue } from '../../shared/aclTabs';
import api from '../../shared/api/api';
import {
  type AclAlias,
  type AclDestination,
  type AclRule,
  AclStatus,
  type AclStatusValue,
  type LicenseInfo,
  type NetworkLocation,
  type ResourceById,
} from '../../shared/api/types';
import { TableValuesListCell } from '../../shared/components/TableValuesListCell/TableValuesListCell';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../shared/defguard-ui/components/Menu/types';
import { Search } from '../../shared/defguard-ui/components/Search/Search';
import { tableEditColumnSize } from '../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { displayDate } from '../../shared/utils/displayDate';
import { canUseBusinessFeature, licenseActionCheck } from '../../shared/utils/license';

type RowData = AclRule;

const columnHelper = createColumnHelper<RowData>();

type Props = {
  license: LicenseInfo | null;
  aliases: ResourceById<AclAlias>;
  destinations: ResourceById<AclDestination>;
  locations: ResourceById<NetworkLocation>;
  data: AclRule[];
  title: string;
  buttonProps: ButtonProps;
  variant: AclListTabValue;
  enableSearch?: boolean;
};

const toggleRulePromise = async (id: number) => {
  const rule = cloneDeep((await api.acl.rule.getRule(id)).data);
  rule.enabled = !rule.enabled;
  return api.acl.rule.editRule(rule);
};

export const RulesTable = ({
  title,
  buttonProps,
  enableSearch,
  aliases,
  destinations,
  locations,
  data,
  license,
  variant,
}: Props) => {
  const navigate = useNavigate();

  const renderResourceBadge = useCallback(
    (marker: 'A' | 'D', key: string, text: string) => (
      <Badge
        className="rules-table-destination-badge"
        data-marker={marker}
        variant={BadgeVariant.Neutral}
        text={text}
        key={key}
      />
    ),
    [],
  );

  const { mutate: deleteRule } = useMutation({
    mutationFn: api.acl.rule.deleteRule,
    meta: {
      invalidate: ['acl'],
    },
  });

  const { mutate: toggleRule } = useMutation({
    mutationFn: toggleRulePromise,
    meta: {
      invalidate: ['acl'],
    },
  });

  const { mutate: deployRule } = useMutation({
    mutationFn: api.acl.rule.applyRules,
    onSuccess: () => {
      Snackbar.default(m.acl_rules_deploy_success());
    },
    meta: {
      invalidate: ['acl'],
    },
  });

  const [search, setSearch] = useState('');

  const renderStatusCell = useCallback(
    (ruleState: AclStatusValue, isEnabled: boolean) => {
      // handle applied rules first
      if (ruleState === AclStatus.Applied) {
        return (
          <TableCell>
            {isEnabled ? (
              <Badge variant={BadgeVariant.Success} text={m.state_active()} />
            ) : (
              <Badge variant={BadgeVariant.Critical} text={m.state_disabled()} />
            )}
          </TableCell>
        );
      }

      if (ruleState === AclStatus.Deleted) {
        return (
          <TableCell>
            <Badge variant={BadgeVariant.Critical} text={m.acl_rules_state_deleted()} />
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
        header: m.acl_rules_col_name(),
        enableSorting: true,
        sortingFn: 'text',
        minSize: 210,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'predefined-destinations',
        header: m.acl_rules_col_predefined_destinations(),
        minSize: 300,
        cell: (info) => {
          const row = info.row.original;
          return (
            <TableCell className="rules-table-destination-cell">
              {row.destinations.map((destinationId) => {
                const destination = destinations[destinationId];
                if (!destination) return null;
                return renderResourceBadge(
                  'D',
                  `destination-${destinationId}`,
                  destination.name,
                );
              })}
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'manual-destinations',
        header: m.acl_rules_col_manual_destination(),
        minSize: 300,
        cell: (info) => {
          const row = info.row.original;

          if (!row.use_manual_destination_settings) {
            return <TableCell className="rules-table-destination-cell" />;
          }

          const manualAddresses = row.addresses.trim();
          const hasManualAddresses = manualAddresses.length > 0;

          return (
            <TableCell>
              {row.any_address ? <span>{m.acl_destination_any_address()}</span> : hasManualAddresses && <span>{manualAddresses}</span>}
              {row.aliases.map((aliasId) => {
                const alias = aliases[aliasId];
                if (!alias) return null;
                return renderResourceBadge('A', `alias-${aliasId}`, alias.name);
              })}
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'locations',
        header: m.acl_rules_col_locations(),
        minSize: 220,
        cell: (info) => {
          const row = info.row.original;
          if (row.all_locations) {
            return (
              <TableCell>
                <Badge
                  variant={BadgeVariant.Success}
                  text={m.acl_rules_all_locations()}
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
      columnHelper.accessor('modified_at', {
        size: 175,
        minSize: 175,
        header: m.edges_col_last_modified(),
        enableSorting: true,
        cell: (info) => (
          <TableCell>
            <span>{displayDate(info.getValue())}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('modified_by', {
        size: 175,
        minSize: 175,
        header: m.edges_col_modified_by(),
        enableSorting: true,
        sortingFn: 'text',
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.display({
        id: 'status',
        header: m.col_status(),
        size: 125,
        minSize: 125,
        enableSorting: false,
        cell: (info) => {
          const row = info.row.original;
          return renderStatusCell(row.state, row.enabled);
        },
      }),
      columnHelper.display({
        id: 'edit',
        header: '',
        size: tableEditColumnSize,
        enableSorting: false,
        enableResizing: false,
        cell: (info) => {
          const row = info.row.original;
          const topItems: MenuItemProps[] = [
            {
              icon: 'edit',
              text: m.controls_edit(),
              onClick: () => {
                licenseActionCheck(canUseBusinessFeature(license), () => {
                  navigate({
                    to: '/acl/edit-rule',
                    search: {
                      rule: row.id,
                      tab: variant,
                    },
                  });
                });
              },
            },
          ];
          switch (variant) {
            case AclListTab.Deployed:
              if (row.enabled) {
                topItems.push({
                  icon: 'disabled',
                  text: m.controls_disable(),
                  onClick: () => {
                    licenseActionCheck(canUseBusinessFeature(license), () => {
                      toggleRule(row.id);
                    });
                  },
                });
              } else {
                topItems.push({
                  icon: 'check',
                  text: m.controls_enable(),
                  onClick: () => {
                    licenseActionCheck(canUseBusinessFeature(license), () => {
                      toggleRule(row.id);
                    });
                  },
                });
              }
              break;
            case AclListTab.Pending:
              topItems.push({
                icon: 'deploy',
                text: m.controls_deploy(),

                onClick: () => {
                  licenseActionCheck(canUseBusinessFeature(license), () => {
                    deployRule([row.id]);
                  });
                },
              });
              break;
          }
          const menuItems: MenuItemsGroup[] = [
            {
              items: topItems,
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
          return <TableEditCell menuItems={menuItems} />;
        },
      }),
    ],
    [
      aliases,
      destinations,
      deleteRule,
      locations,
      navigate,
      renderStatusCell,
      renderResourceBadge,
      license,
      variant,
      toggleRule,
      deployRule,
    ],
  );

  const visibleRules = useMemo(() => {
    let res = data;
    const query = search.trim().toLowerCase();
    if (query.length > 0) {
      res = res.filter(
        (rule) =>
          rule.name.toLowerCase().includes(query) ||
          rule.modified_by.toLowerCase().includes(query),
      );
    }
    return res;
  }, [search, data]);

  const table = useReactTable({
    initialState: {
      sorting: [
        {
          id: 'name',
          desc: false,
        },
      ],
    },
    columns,
    data: visibleRules,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
    getSortedRowModel: getSortedRowModel(),
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
