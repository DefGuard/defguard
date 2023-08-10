import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { isUndefined } from 'lodash-es';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import ConfirmModal, {
  ConfirmModalType,
} from '../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';
import { useDeleteDeviceModal } from '../../hooks/useDeleteDeviceModal';

export const DeleteUserDeviceModal = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const [device, visible] = useDeleteDeviceModal(
    (state) => [state.device, state.visible],
    shallow,
  );
  const [setModalState, closeModal] = useDeleteDeviceModal(
    (state) => [state.setState, state.close],
    shallow,
  );
  const {
    device: { deleteDevice },
  } = useApi();
  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation(
    [MutationKeys.DELETE_USER_DEVICE],
    deleteDevice,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
        toaster.success(LL.modals.deleteDevice.messages.success());
        closeModal();
      },
      onError: (err: AxiosError) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  return (
    <ConfirmModal
      title={LL.modals.deleteDevice.title()}
      type={ConfirmModalType.WARNING}
      subTitle={LL.modals.deleteDevice.message({
        deviceName: device?.name || '',
      })}
      cancelText={LL.form.cancel()}
      submitText={LL.modals.deleteDevice.submit()}
      loading={isLoading || isUndefined(device)}
      isOpen={visible}
      setIsOpen={(visibility) => setModalState({ visible: visibility })}
      onSubmit={() => {
        if (device) {
          mutate(device);
        }
      }}
    />
  );
};
