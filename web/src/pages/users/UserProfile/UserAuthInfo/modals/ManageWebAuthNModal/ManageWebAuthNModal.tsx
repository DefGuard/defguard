import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';

import MessageBox from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';
import { RegisterWebAuthNForm } from './components/RegisterWebAuthNForm';
import { WebAuthNKeyRow } from './components/WebAuthNKeyRow';

export const ManageWebAuthNKeysModal = () => {
  const user = useUserProfileStore((state) => state.user);
  const modalState = useModalStore((state) => state.manageWebAuthNKeysModal);
  const setModalState = useModalStore((state) => state.setState);

  const {
    auth: {
      mfa: {
        webauthn: { deleteKey },
      },
    },
  } = useApi();
  const toaster = useToaster();
  const queryClient = useQueryClient();
  const { mutate: deleteKeyMutation, isLoading: deleteKeyLoading } =
    useMutation([MutationKeys.WEBUAUTHN_DELETE_KEY], deleteKey, {
      onSuccess: () => {
        toaster.success('WebAuthN key deleted.');
        queryClient.invalidateQueries([QueryKeys.FETCH_USER]);
      },
      onError: (err) => {
        console.error(err);
        toaster.error('Key deletion failed.');
      },
    });

  return (
    <ModalWithTitle
      backdrop
      id="manage-webauthn-modal"
      title="Security keys"
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalState({ manageWebAuthNKeysModal: { visible: visibility } })
      }
    >
      <MessageBox>
        <p>
          Security keys can be used as your second factor of authentication
          instead of a verification code. Learn more about configuring a
          security key.
        </p>
      </MessageBox>
      {user && user.security_keys.length > 0 && (
        <div className="current-keys">
          {user?.security_keys.map((key) => (
            <WebAuthNKeyRow
              key={key.id}
              data={key}
              onDelete={() => {
                if (user) {
                  deleteKeyMutation({
                    username: user.username,
                    keyId: key.id,
                  });
                }
              }}
              disableDelete={deleteKeyLoading}
            />
          ))}
        </div>
      )}
      <RegisterWebAuthNForm />
    </ModalWithTitle>
  );
};
