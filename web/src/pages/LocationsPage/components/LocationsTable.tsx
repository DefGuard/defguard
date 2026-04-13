import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { NetworkLocation } from '../../../shared/api/types';
import { GatewaysStatusBadge } from '../../../shared/components/GatewaysStatusBadge/GatewaysStatusBadge';
import { TableValuesListCell } from '../../../shared/components/TableValuesListCell/TableValuesListCell';
import { Badge } from '../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { Search } from '../../../shared/defguard-ui/components/Search/Search';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { tableEditColumnSize } from '../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { TableEditCell } from '../../../shared/defguard-ui/components/table/TableEditCell/TableEditCell';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing, ThemeVariable } from '../../../shared/defguard-ui/types';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import {
  getLicenseInfoQueryOptions,
  getLocationsQueryOptions,
} from '../../../shared/query';
import { tableSortingFns } from '../../../shared/utils/dateSortingFn';
import {
  canUseEnterpriseFeature,
  licenseActionCheck,
} from '../../../shared/utils/license';
import { useGatewayWizardStore } from '../../GatewaySetupPage/useGatewayWizardStore';

type RowData = NetworkLocation;

const columnHelper = createColumnHelper<RowData>();

export const LocationsTable = () => {
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);
  const { data: license } = useSuspenseQuery(getLicenseInfoQueryOptions);
  const navigate = useNavigate();
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
      text: m.location_add(),
      iconLeft: 'add-location',
      testId: 'add-location',
      onClick: () => {
        if (
          license?.limits &&
          license.limits.locations.current === license.limits.locations.limit
        ) {
          openModal(ModalName.LimitReached);
        } else {
          openModal(ModalName.AddLocation, {
            license,
          });
        }
      },
    }),
    [license],
  );

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.form_label_name(),
        enableSorting: true,
        sortingFn: 'text',
        minSize: 200,
        meta: {
          flex: true,
        },
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('gateways', {
        header: m.location_col_gateway_status(),
        size: 175,
        minSize: 175,
        cell: (info) => (
          <TableCell>
            <GatewaysStatusBadge data={info.getValue() ?? []} />
          </TableCell>
        ),
      }),
      columnHelper.accessor('endpoint', {
        header: m.location_col_gateway_ip(),
        size: 200,
        minSize: 200,
        cell: (info) => {
          return (
            <TableCell>
              <span>{info.getValue()}</span>
            </TableCell>
          );
        },
      }),
      columnHelper.accessor('acl_enabled', {
        header: m.cmp_nav_group_firewall(),
        minSize: 100,
        cell: (info) => (
          <TableCell className="cell-with-check-icons">
            {info.getValue() ? (
              <Icon icon="check-filled" staticColor={ThemeVariable.FgSuccess} />
            ) : (
              <Icon icon="disabled" />
            )}
          </TableCell>
        ),
      }),
      columnHelper.accessor('address', {
        header: m.location_col_vpn_network(),
        minSize: 250,
        cell: (info) => <TableValuesListCell values={info.getValue()} />,
      }),
      columnHelper.accessor('location_mfa_mode', {
        header: m.location_col_mfa(),
        minSize: 100,
        sortingFn: 'text',
        cell: (info) => {
          switch (info.getValue()) {
            case 'disabled':
              return (
                <TableCell>
                  <Badge text={m.location_mfa_none()} />
                </TableCell>
              );
            case 'external':
              return (
                <TableCell>
                  <Badge
                    icon="external-mfa"
                    text={m.location_mfa_external()}
                    variant="warning"
                  />
                </TableCell>
              );
            case 'internal':
              return (
                <TableCell>
                  <Badge
                    icon="internal-mfa"
                    text={m.location_mfa_internal()}
                    variant="success"
                  />
                </TableCell>
              );
          }
        },
      }),
      columnHelper.accessor('service_location_mode', {
        header: m.location_col_type(),
        minSize: 100,
        enableSorting: true,
        sortingFn: 'text',
        cell: (info) => {
          switch (info.getValue()) {
            case 'disabled':
              return (
                <TableCell>
                  <span>{m.location_type_regular()}</span>
                </TableCell>
              );
            case 'prelogon':
              return (
                <TableCell>
                  <span>{m.location_type_prelogon()}</span>
                </TableCell>
              );
            case 'alwayson':
              return (
                <TableCell>
                  <span>{m.location_type_always()}</span>
                </TableCell>
              );
          }
        },
      }),
      columnHelper.accessor('fwmark', {
        header: m.location_col_fwmark(),
        minSize: 75,
        cell: (info) => (
          <TableCell>
            <span>0x{info.getValue().toString(16)}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('mtu', {
        header: m.location_col_mtu(),
        minSize: 75,
        cell: (info) => (
          <TableCell>
            <span>{info.getValue()}</span>
          </TableCell>
        ),
      }),
      columnHelper.accessor('allowed_groups', {
        header: m.location_col_allowed_groups(),
        size: 400,
        minSize: 200,
        cell: (info) => {
          const value = info.getValue();
          const len = value?.length ?? 0;
          const allowAllGroups = info.row.original.allow_all_groups;
          return (
            <TableCell>
              {allowAllGroups && (
                <Badge
                  showIcon
                  icon="status-available"
                  variant="success"
                  text={m.location_allowed_groups_all()}
                />
              )}
              {!allowAllGroups && len > 0 && <span>{info.getValue()?.join(', ')}</span>}
            </TableCell>
          );
        },
      }),
      columnHelper.display({
        id: 'edit',
        size: tableEditColumnSize,
        header: '',
        enableResizing: false,
        cell: (info) => {
          const row = info.row.original;
          return (
            <TableEditCell
              menuItems={[
                {
                  items: [
                    {
                      icon: 'edit',
                      text: m.controls_edit(),
                      onClick: () => {
                        navigate({
                          to: '/locations/$locationId/edit',
                          params: {
                            locationId: row.id.toString(),
                          },
                        });
                      },
                    },
                    {
                      icon: 'network-settings',
                      text: m.location_action_gateway_setup(),
                      onClick: async () => {
                        // allow 1 gateway per location if below business tier
                        const action = () => {
                          useGatewayWizardStore.getState().start({ network_id: row.id });
                          navigate({
                            to: '/setup-gateway',
                          });
                        };
                        if (row.gateways.length >= 1) {
                          licenseActionCheck(canUseEnterpriseFeature(license), action);
                        } else {
                          action();
                        }
                      },
                    },
                  ],
                },
                {
                  items: [
                    {
                      icon: 'delete',
                      text: m.controls_delete(),
                      variant: 'danger',
                      onClick: () => {
                        openModal(ModalName.ConfirmAction, {
                          title: m.modal_delete_location_title(),
                          contentMd: m.modal_delete_location_body({ name: row.name }),
                          actionPromise: () => api.location.deleteLocation(row.id),
                          invalidateKeys: [['network'], ['enterprise_info']],
                          submitProps: { text: m.controls_delete(), variant: 'critical' },
                          onSuccess: () => Snackbar.default(m.location_delete_success()),
                          onError: () => Snackbar.error(m.location_delete_failed()),
                        });
                      },
                    },
                  ],
                },
              ]}
            />
          );
        },
      }),
    ],
    [navigate, license],
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
    columnResizeMode: 'onChange',
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <>
      {locations.length > 0 && (
        <>
          <SizedBox height={ThemeSpacing.Xl3} />
          <TableTop text={m.locations_management_title()}>
            <Search
              placeholder={m.controls_search()}
              onChange={setSearch}
              initialValue={search}
            />
            <Button {...addButtonProps} />
          </TableTop>
          {transformedData.length > 0 && <TableBody table={table} />}
          {transformedData.length === 0 && (
            <EmptyStateFlexible
              icon="search"
              title={m.search_empty_common_title()}
              subtitle={m.search_empty_common_subtitle()}
            />
          )}
        </>
      )}
      {locations.length === 0 && (
        <EmptyStateFlexible
          title={m.locations_empty_title()}
          subtitle={m.locations_empty_subtitle()}
          primaryAction={addButtonProps}
        />
      )}
    </>
  );
};
