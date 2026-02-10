import { useMutation } from '@tanstack/react-query';
import {
  createColumnHelper,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table';
import { groupBy } from 'lodash-es';
import { useMemo } from 'react';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import type { AuthKey, AuthKeyTypeValue } from '../../../../../shared/api/types';
import { IconButtonMenu } from '../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../../../../shared/defguard-ui/components/Menu/types';
import { tableEditColumnSize } from '../../../../../shared/defguard-ui/components/table/consts';
import { TableBody } from '../../../../../shared/defguard-ui/components/table/TableBody/TableBody';
import { TableCell } from '../../../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { useClipboard } from '../../../../../shared/defguard-ui/hooks/useClipboard';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { downloadText } from '../../../../../shared/utils/download';
import { formatFileName } from '../../../../../shared/utils/formatFileName';
import { useUserProfile } from '../../hooks/useUserProfilePage';

type RowData = {
  id: number;
  name: string;
  key?: string;
  key_type?: AuthKeyTypeValue;
  sshKey?: string;
  gpgKey?: string;
};

// provisioning a yubikey makes 2 keys but we want to show this keys as one
const mapData = (data: AuthKey[]): RowData[] => {
  // single keys first
  const res: RowData[] = data
    .filter((key) => !isPresent(key.yubikey_id))
    .map(
      (key): RowData => ({
        id: key.id,
        name: key.name as string,
        key: key.key,
        key_type: key.key_type,
      }),
    );
  // group and merge yubi keys
  const yubiKeys = data.filter((key) => isPresent(key.yubikey_id));
  const yubiGrouped = groupBy(yubiKeys, 'yubikey_id');
  const groupedKeys = Object.keys(yubiGrouped);
  for (const objKey of groupedKeys) {
    const keys = yubiGrouped[objKey];
    const name = keys[0].yubikey_name as string;
    const id = keys[0].yubikey_id as number;
    let sshKey: string;
    let gpgKey: string;
    if (keys[0].key_type === 'ssh') {
      sshKey = keys[0].key as string;
      gpgKey = keys[1].key as string;
    } else {
      gpgKey = keys[0].key as string;
      sshKey = keys[1].key as string;
    }
    res.push({
      id,
      name,
      sshKey,
      gpgKey,
    });
  }
  return res;
};

const columnHelper = createColumnHelper<RowData>();

export const ProfileAuthKeysTable = () => {
  const { writeToClipboard } = useClipboard();
  const username = useUserProfile((s) => s.user.username);

  const authKeys = useUserProfile((s) => s.authKeys);
  const mapped = useMemo(() => mapData(authKeys), [authKeys]);

  const { mutate: deleteAuthKey } = useMutation({
    mutationFn: api.user.deleteAuthKey,
    meta: {
      invalidate: [['user-overview'], ['user', username, 'auth_key']],
    },
  });

  const columns = useMemo(
    () => [
      columnHelper.accessor('name', {
        header: m.profile_auth_keys_table_col_name(),
        enableSorting: true,
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
        id: 'edit',
        size: tableEditColumnSize,
        header: '',
        enableSorting: false,
        cell: (info) => {
          const rowData = info.row.original;
          const { sshKey, gpgKey } = rowData;
          const itemsProps: MenuItemProps[] = [];
          const fileName = formatFileName(rowData.name);
          if (isPresent(gpgKey) && isPresent(sshKey)) {
            itemsProps.push({
              icon: 'download',
              text: `${m.controls_download()} SSH`,
              onClick: () => {
                downloadText(sshKey, `${fileName}_ssh`, 'pub');
              },
            });
            itemsProps.push({
              icon: 'copy',
              text: `${m.controls_copy()} SSH`,
              onClick: () => {
                void writeToClipboard(sshKey);
              },
            });
            itemsProps.push({
              icon: 'download',
              text: `${m.controls_download()} GPG`,
              onClick: () => {
                downloadText(sshKey, `${fileName}_gpg`, 'pub');
              },
            });
            itemsProps.push({
              icon: 'copy',
              text: `${m.controls_copy()} GPG`,
              onClick: () => {
                void writeToClipboard(gpgKey);
              },
            });
          } else {
            itemsProps.push({
              icon: 'download',
              text: m.controls_download(),
              onClick: () => {
                if (rowData.key_type === 'gpg') {
                  downloadText(rowData.key as string, `${fileName}_gpg`, 'pub');
                } else {
                  downloadText(rowData.key as string, `${fileName}_ssh`, 'pub');
                }
              },
            });
            itemsProps.push({
              icon: 'copy',
              text: `${m.controls_copy()}`,
              onClick: () => {
                void writeToClipboard(rowData.key as string);
              },
            });
          }
          const menuItems: MenuItemsGroup[] = [
            {
              items: [
                ...itemsProps,
                {
                  icon: 'edit',
                  text: m.controls_rename(),
                  onClick: () => {
                    openModal(ModalName.RenameAuthKey, {
                      id: rowData.id,
                      name: rowData.name,
                      username,
                    });
                  },
                },
                {
                  icon: 'delete',
                  testId: 'delete-key',
                  variant: 'danger',
                  text: m.controls_delete(),
                  onClick: () => {
                    deleteAuthKey({
                      id: rowData.id,
                      username,
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
    [deleteAuthKey, username, writeToClipboard],
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
    columns,
    data: mapped,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableSorting: true,
    enableRowSelection: false,
    columnResizeMode: 'onChange',
  });

  return <TableBody table={table} />;
};
