import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useDeleteAuthenticationKeyModal } from './useDeleteAuthenticationKeyModal';

export const DeleteAuthenticationKeyModal = () => {
  const {
    user: { deleteAuthenticationKey, deleteYubiKey },
  } = useApi();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const isOpen = useDeleteAuthenticationKeyModal((s) => s.visible);
  const [close, reset] = useDeleteAuthenticationKeyModal(
    (s) => [s.close, s.reset],
    shallow,
  );
  const keyData = useDeleteAuthenticationKeyModal((s) => s.keyData);

  const onSuccess = () => {
    toaster.success(LL.messages.success());
    void queryClient.invalidateQueries({
      queryKey: [QueryKeys.FETCH_AUTHENTICATION_KEYS_INFO],
    });
    close();
  };

  const onError = (e: AxiosError) => {
    toaster.error(LL.messages.error());
    console.error(e);
  };

  const { mutate: deleteYubikeyMutation, isPending: yubikeyPending } = useMutation({
    mutationFn: deleteYubiKey,
    onSuccess,
    onError,
  });

  const { mutate: deleteAuthenticationKeyMutation, isPending: authKeyPending } =
    useMutation({
      mutationFn: deleteAuthenticationKey,
      onSuccess,
      onError,
    });

  return (
    <ConfirmModal
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      type={ConfirmModalType.WARNING}
      title={LL.userPage.authenticationKeys.deleteModal.title()}
      subTitle={LL.userPage.authenticationKeys.deleteModal.confirmMessage({
        name: keyData?.name ?? '',
      })}
      cancelText={LL.form.cancel()}
      submitText={LL.common.controls.delete()}
      onSubmit={() => {
        if (keyData) {
          if (keyData.type === 'yubikey') {
            deleteYubikeyMutation({
              id: keyData.id,
              username: keyData.username,
            });
          } else {
            deleteAuthenticationKeyMutation({
              id: keyData.id,
              username: keyData.username,
            });
          }
        }
      }}
      loading={authKeyPending || yubikeyPending}
    />
  );
};
