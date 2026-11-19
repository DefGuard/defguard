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

  const { mutate: enableDevices } = useMutation({
    mutationFn: api.device.bulkEnableDevices,
    meta: { invalidate: ['device'] },
    onError: () => Snackbar.error(m.devices_enable_error()),
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
      const toBlock = devices.filter((device) => device.state !== DeviceState.Blocked);
      const toEnable = devices.filter((device) => device.state === DeviceState.Blocked);
      const blockIds = toBlock.map(({ id }) => id);
      const enableIds = toEnable.map(({ id }) => id);
      const deleteIds = devices.map(({ id }) => id);

      const withSuccess = (message: string) => ({
        onSuccess: () => {
          onSuccess?.();
          Snackbar.default(message);
        },
      });

      const handleVerify = () => {
        verifyDevices(verifyIds, withSuccess(m.devices_verify_success()));
      };

      const handleRevoke = () => {
        revokeVerification(
          revokeIds,
          withSuccess(m.devices_revoke_verification_success()),
        );
      };

      const handleEnable = () => {
        enableDevices(enableIds, withSuccess(m.devices_enable_success()));
      };

      const handleBlock = () => {
        const single = toBlock.length === 1;
        openModal(ModalName.ConfirmAction, {
          title: single
            ? m.devices_modal_block_single_title()
            : m.devices_modal_block_title(),
          contentMd: single
            ? m.devices_modal_block_single_content({ name: toBlock[0].name })
            : m.devices_modal_block_content({ count: toBlock.length }),
          actionPromise: () => api.device.bulkBlockDevices(blockIds),
          invalidateKeys: [['device']],
          submitProps: { text: m.devices_action_block(), variant: 'critical' },
          onSuccess: () => {
            onSuccess?.();
            Snackbar.default(m.devices_block_success());
          },
          onError: () => Snackbar.error(m.devices_block_error()),
        });
      };

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

      const verificationItems: MenuItemProps[] = [];
      if (isEnterprise && showRevoke) {
        verificationItems.push({
          text: m.devices_action_revoke_verification(),
          icon: 'close',
          testId: `${testIdPrefix}-revoke-verification`,
          onClick: handleRevoke,
        });
      } else if (isEnterprise && verifyIds.length > 0) {
        verificationItems.push({
          text: m.devices_action_verify(),
          icon: 'device-attestation',
          testId: `${testIdPrefix}-verify`,
          onClick: handleVerify,
        });
      }

      const stateItems: MenuItemProps[] = [];
      if (enableIds.length > 0) {
        stateItems.push({
          text: m.devices_action_enable(),
          icon: 'check',
          testId: `${testIdPrefix}-enable`,
          onClick: handleEnable,
        });
      }
      if (blockIds.length > 0) {
        stateItems.push({
          text: m.devices_action_block(),
          icon: 'cancel',
          testId: `${testIdPrefix}-block`,
          onClick: handleBlock,
        });
      }

      const deleteItems: MenuItemProps[] =
        deleteIds.length > 0
          ? [
              {
                text: m.controls_delete(),
                icon: 'delete',
                variant: 'danger',
                testId: `${testIdPrefix}-delete`,
                onClick: handleDelete,
              },
            ]
          : [];

      const groups: MenuItemsGroup[] = [
        { items: [...verificationItems, ...stateItems] },
        { items: deleteItems },
      ];
      return groups.filter((group) => group.items.length > 0);
    },
    [verifyDevices, revokeVerification, enableDevices, isEnterprise],
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
