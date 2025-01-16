import { useMutation, useQueryClient } from '@tanstack/react-query';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useDeleteStandaloneDeviceModal } from '../../../hooks/useDeleteStandaloneDeviceModal';

export const ConfirmDeviceDeleteModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.deleteStandaloneDevice;
  const [visible, device] = useDeleteStandaloneDeviceModal(
    (s) => [s.visible, s.device],
    shallow,
  );
  const queryClient = useQueryClient();
  const [close, reset] = useDeleteStandaloneDeviceModal(
    (s) => [s.close, s.reset],
    shallow,
  );

  const {
    standaloneDevice: { deleteDevice },
  } = useApi();

  const toaster = useToaster();

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: deleteDevice,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      });
      close();
      toaster.success(localLL.messages.success());
    },
    onError: (e) => {
      toaster.error(localLL.messages.error());
      close();
      console.error(e);
    },
  });

  const isOpen = visible && device !== undefined;

  return (
    <ConfirmModal
      isOpen={isOpen}
      title={localLL.title()}
      subTitle={localLL.content({
        name: (device?.name as string) ?? '',
      })}
      submitText={LL.common.controls.delete()}
      cancelText={LL.common.controls.cancel()}
      onSubmit={() => {
        if (device) {
          mutate(device.id);
        }
      }}
      onClose={close}
      afterClose={reset}
      loading={isLoading}
    />
  );
};
