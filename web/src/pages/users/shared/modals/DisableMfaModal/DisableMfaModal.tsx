import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../../shared/utils/invalidateMultipleQueries';
import { useDisableMfaModal } from './store';

export const DisableMfaModal = () => {
  const {
    user: { disableUserMfa },
  } = useApi();

  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const [user, isOpen, setIsOpen, closeModal] = useDisableMfaModal(
    (state) => [state.user, state.visible, state.setIsOpen, state.close],
    shallow,
  );

  const toaster = useToaster();

  const { mutate, isPending } = useMutation({
    mutationFn: disableUserMfa,
    onSuccess: () => {
      toaster.success(
        LL.modals.disableMfa.messages.success({ username: user?.username || '' }),
      );
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_USERS_LIST],
        [QueryKeys.FETCH_USER_PROFILE],
      ]);
      closeModal();
      navigate('/admin/users', { replace: true });
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  return (
    <ConfirmModal
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      type={ConfirmModalType.WARNING}
      title={LL.modals.disableMfa.title()}
      subTitle={LL.modals.disableMfa.message({
        username: user?.username || '',
      })}
      cancelText={LL.modals.disableMfa.controls.cancel()}
      submitText={LL.modals.disableMfa.controls.submit()}
      onSubmit={() => {
        if (user) {
          mutate(user.username);
        }
      }}
      loading={isPending}
    />
  );
};
