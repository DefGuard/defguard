import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';
import { RegisterWebAuthNForm } from './components/RegisterWebAuthNForm';
import { WebAuthNKeyRow } from './components/WebAuthNKeyRow';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';

export const ManageWebAuthNKeysModal = () => {
  const { LL } = useI18nContext();
  const userProfile = useUserProfileStore((state) => state.userProfile);
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
  const { mutate: deleteKeyMutation, isLoading: deleteKeyLoading } = useMutation(
    [MutationKeys.WEBUAUTHN_DELETE_KEY],
    deleteKey,
    {
      onSuccess: () => {
        toaster.success(LL.modals.manageWebAuthNKeys.messages.deleted());
        queryClient.invalidateQueries([QueryKeys.FETCH_USER_PROFILE]);
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  return (
    <ModalWithTitle
      backdrop
      id="manage-webauthn-modal"
      title={LL.modals.manageWebAuthNKeys.title()}
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalState({ manageWebAuthNKeysModal: { visible: visibility } })
      }
    >
      <MessageBox type={MessageBoxType.INFO}>
        {parse(LL.modals.manageWebAuthNKeys.infoMessage())}
      </MessageBox>
      {userProfile && userProfile.security_keys.length > 0 && (
        <div className="current-keys">
          {userProfile?.security_keys.map((key) => (
            <WebAuthNKeyRow
              key={key.id}
              data={key}
              onDelete={() => {
                if (userProfile) {
                  deleteKeyMutation({
                    username: userProfile.user.username,
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
