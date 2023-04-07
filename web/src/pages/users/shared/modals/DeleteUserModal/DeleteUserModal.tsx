import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import ConfirmModal, {
  ConfirmModalType,
} from '../../../../../shared/components/layout/ConfirmModal/ConfirmModal';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { User } from '../../../../../shared/types';

export const DeleteUserModal = () => {
  const {
    user: { deleteUser },
  } = useApi();

  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();

  const [modalState, setModalState] = useModalStore(
    (state) => [state.deleteUserModal, state.setDeleteUserModal],
    shallow
  );

  const toaster = useToaster();

  const { mutate, isLoading } = useMutation((user: User) => deleteUser(user), {
    onSuccess: (_, variables) => {
      toaster.success(
        LL.modals.deleteUser.messages.success({ username: variables.username })
      );
      queryClient.invalidateQueries([QueryKeys.FETCH_USERS]);
      queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
      setModalState({ visible: false, user: undefined });
      navigate('/admin/users', { replace: true });
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      setModalState({ visible: false, user: undefined });
      console.error(err);
    },
  });

  return (
    <ConfirmModal
      isOpen={modalState.visible}
      setIsOpen={(v: boolean) => setModalState({ visible: v })}
      type={ConfirmModalType.WARNING}
      title={LL.modals.deleteUser.title()}
      subTitle={LL.modals.deleteUser.message({
        username: modalState.user?.username || '',
      })}
      cancelText={LL.form.cancel()}
      submitText={LL.modals.deleteUser.controls.submit()}
      onSubmit={() => {
        if (modalState.user) {
          mutate(modalState.user);
        }
      }}
      loading={isLoading}
    />
  );
};
