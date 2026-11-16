import { useMutation } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import {
  type DeviceOverview,
  DeviceState,
  DeviceVerification,
} from '../../../shared/api/types';
import type {
  MenuItemProps,
  MenuItemsGroup,
} from '../../../shared/defguard-ui/components/Menu/types';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import {
  type UserDeviceContext,
  userDeviceActions,
} from '../../../shared/utils/userDeviceActions';
import { useEnterpriseTier } from './useEnterpriseTier';

type DeviceActionsTarget = {
  devices: DeviceOverview[];
  testIdPrefix: string;
  onSuccess?: () => void;
};

export const useDeviceActions = () => {
  const isEnterprise = useEnterpriseTier();

  const { mutate: verifyDevices } = useMutation({
    mutationFn: api.device.bulkVerifyDevices,
    meta: { invalidate: ['device'] },
    onError: () => Snackbar.error(m.devices_verify_error()),
  });

  const { mutate: revokeVerification } = useMutation({
    mutationFn: api.device.bulkRevokeDeviceVerification,
    meta: { invalidate: ['device'] },
    onError: () => Snackbar.error(m.devices_revoke_verification_error()),
  });

  const { mutate: blockDevices } = useMutation({
    mutationFn: api.device.bulkBlockDevices,
    meta: { invalidate: ['device'] },
    onError: () => Snackbar.error(m.devices_block_error()),
  });

  const getMenuItems = useCallback(
    ({ devices, testIdPrefix, onSuccess }: DeviceActionsTarget): MenuItemsGroup[] => {
      const verifyIds = devices
        .filter((device) => device.verification !== DeviceVerification.Verified)
        .map(({ id }) => id);
      const revokeIds = devices
        .filter((device) => device.verification === DeviceVerification.Verified)
        .map(({ id }) => id);
      const showRevoke = revokeIds.length > 0 && verifyIds.length === 0;
      const blockIds = devices
        .filter((device) => device.state !== DeviceState.Blocked)
        .map(({ id }) => id);
      const deleteIds = devices.map(({ id }) => id);

      const handleDelete = () => {
        const single = devices.length === 1;
        openModal(ModalName.ConfirmAction, {
          title: single
            ? m.devices_modal_delete_single_title()
            : m.devices_modal_delete_title(),
          contentMd: single
            ? m.devices_modal_delete_single_content({ name: devices[0].name })
            : m.devices_modal_delete_content({ count: deleteIds.length }),
          actionPromise: () => api.device.bulkDeleteDevices(deleteIds),
          invalidateKeys: [['device']],
          submitProps: { text: m.controls_delete(), variant: 'critical' },
          onSuccess: () => {
            onSuccess?.();
            Snackbar.default(m.devices_delete_success());
          },
          onError: () => Snackbar.error(m.devices_delete_error()),
        });
      };

      const verificationItem: MenuItemProps = showRevoke
        ? {
            text: m.devices_action_revoke_verification(),
            icon: 'close',
            testId: `${testIdPrefix}-revoke-verification`,
            onClick: () =>
              revokeVerification(revokeIds, {
                onSuccess: () => {
                  onSuccess?.();
                  Snackbar.default(m.devices_revoke_verification_success());
                },
              }),
          }
        : {
            text: m.devices_action_verify(),
            icon: 'device-attestation',
            testId: `${testIdPrefix}-verify`,
            disabled: verifyIds.length === 0,
            onClick: () =>
              verifyDevices(verifyIds, {
                onSuccess: () => {
                  onSuccess?.();
                  Snackbar.default(m.devices_verify_success());
                },
              }),
          };

      return [
        {
          items: [
            ...(isEnterprise ? [verificationItem] : []),
            {
              text: m.devices_action_block(),
              icon: 'cancel',
              testId: `${testIdPrefix}-block`,
              disabled: blockIds.length === 0,
              onClick: () =>
                blockDevices(blockIds, {
                  onSuccess: () => {
                    onSuccess?.();
                    Snackbar.default(m.devices_block_success());
                  },
                }),
            },
          ],
        },
        {
          items: [
            {
              text: m.controls_delete(),
              icon: 'delete',
              variant: 'danger',
              testId: `${testIdPrefix}-delete`,
              disabled: deleteIds.length === 0,
              onClick: handleDelete,
            },
          ],
        },
      ];
    },
    [verifyDevices, revokeVerification, blockDevices, isEnterprise],
  );

  const getRowMenuItems = useCallback(
    (device: DeviceOverview): MenuItemsGroup[] => {
      const actionGroups = getMenuItems({ devices: [device], testIdPrefix: 'device' });
      const { username } = device;

      const withContext = (action: (context: UserDeviceContext) => void) => async () => {
        const context = await userDeviceActions.fetchContext(username, device.id);
        if (context) action(context);
      };

      return [
        {
          items: [
            {
              text: m.controls_edit(),
              icon: 'edit',
              testId: 'device-edit-device',
              onClick: withContext((context) =>
                userDeviceActions.openEdit(username, context),
              ),
            },
            {
              text: m.profile_devices_menu_ip_settings(),
              icon: 'gateway',
              testId: 'assign-device-ip',
              onClick: withContext(({ device }) =>
                userDeviceActions.openIpSettings(username, device),
              ),
            },
            {
              text: m.profile_devices_menu_show_config(),
              icon: 'config',
              testId: 'device-show-config',
              onClick: withContext(({ device }) => userDeviceActions.openConfig(device)),
            },
          ],
        },
        ...actionGroups,
      ];
    },
    [getMenuItems],
  );

  return { getMenuItems, getRowMenuItems };
};
