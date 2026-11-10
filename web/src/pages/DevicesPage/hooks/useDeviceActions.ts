import { useMutation } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import {
  type DeviceOverview,
  DeviceState,
  DeviceVerification,
} from '../../../shared/api/types';
import type { MenuItemsGroup } from '../../../shared/defguard-ui/components/Menu/types';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';

type DeviceActionsTarget = {
  devices: DeviceOverview[];
  testIdPrefix: string;
  onSuccess?: () => void;
};

export const useDeviceActions = () => {
  const { mutate: verifyDevices } = useMutation({
    mutationFn: api.device.bulkVerifyDevices,
    meta: { invalidate: ['device'] },
    onError: () => Snackbar.error(m.devices_verify_error()),
  });

  const { mutate: blockDevices } = useMutation({
    mutationFn: api.device.bulkBlockDevices,
    meta: { invalidate: ['device'] },
    onError: () => Snackbar.error(m.devices_block_error()),
  });

  return useCallback(
    ({ devices, testIdPrefix, onSuccess }: DeviceActionsTarget): MenuItemsGroup[] => {
      const verifyIds = devices
        .filter((device) => device.verification !== DeviceVerification.Verified)
        .map(({ id }) => id);
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

      return [
        {
          items: [
            {
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
            },
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
    [verifyDevices, blockDevices],
  );
};
