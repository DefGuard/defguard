import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useDeleteApiTokenModal } from './useDeleteApiTokenModal';

export const DeleteApiTokenModal = () => {
  const {
    user: { deleteApiToken },
  } = useApi();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const isOpen = useDeleteApiTokenModal((s) => s.visible);
  const [close, reset] = useDeleteApiTokenModal((s) => [s.close, s.reset], shallow);
  const keyData = useDeleteApiTokenModal((s) => s.tokenData);

  const onSuccess = () => {
    toaster.success(LL.messages.success());
    void queryClient.invalidateQueries({
      queryKey: [QueryKeys.FETCH_API_TOKENS_INFO],
    });
    close();
  };

  const onError = (e: AxiosError) => {
    toaster.error(LL.messages.error());
    console.error(e);
  };

  const { mutate: deleteApiTokenMutation, isPending: apiTokenPending } = useMutation({
    mutationFn: deleteApiToken,
    onSuccess,
    onError,
  });

  return (
    <ConfirmModal
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      type={ConfirmModalType.WARNING}
      title={LL.userPage.apiTokens.deleteModal.title()}
      subTitle={LL.userPage.apiTokens.deleteModal.confirmMessage({
        name: keyData?.name ?? '',
      })}
      cancelText={LL.form.cancel()}
      submitText={LL.common.controls.delete()}
      onSubmit={() => {
        if (keyData) {
          deleteApiTokenMutation({
            id: keyData.id,
            username: keyData.username,
          });
        }
      }}
      loading={apiTokenPending || apiTokenPending}
    />
  );
};
