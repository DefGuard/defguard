import './style.scss';

import { useMutation } from '@tanstack/react-query';
import clipboard from 'clipboardy';
import { saveAs } from 'file-saver';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { ModalWithTitle } from '../../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import {
  IconCopy,
  IconDownload,
} from '../../../../../../shared/components/svg';
import { useAuthStore } from '../../../../../../shared/hooks/store/useAuthStore';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../../shared/mutations';

export const RecoveryCodesModal = () => {
  const modalState = useModalStore((state) => state.recoveryCodesModal);
  const setModalState = useModalStore((state) => state.setRecoveryCodesModal);

  return (
    <ModalWithTitle
      id="view-recovery-codes"
      title="Recovery codes"
      isOpen={modalState.visible}
      setIsOpen={(visible) => setModalState({ visible, codes: undefined })}
      disableClose={true}
      backdrop
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const codes = useModalStore((state) => state.recoveryCodesModal.codes);
  const setModalState = useModalStore((state) => state.setRecoveryCodesModal);
  const {
    auth: {
      mfa: { enable },
    },
  } = useApi();
  const logOut = useAuthStore((state) => state.logOut);
  const { mutate, isLoading } = useMutation([MutationKeys.ENABLE_MFA], enable, {
    onSuccess: () => {
      setModalState({ visible: false, codes: undefined });
      logOut();
    },
  });
  const toaster = useToaster();

  if (!codes) return null;

  return (
    <>
      <MessageBox type={MessageBoxType.INFO}>
        <p>
          Treat your recovery codes with the same level of attention as you
          would your password! We recommend saving them with a password manager
          such as Lastpass, bitwarden or Keeper.
        </p>
      </MessageBox>
      <div className="codes">
        {codes.map((code) => (
          <p key={code}>{code}</p>
        ))}
      </div>
      <div className="actions">
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconDownload />}
          text="Download"
          onClick={() => {
            if (codes) {
              const blob = new Blob([codes.join('\n')], {
                type: 'text/plain;charset=utf-8',
              });
              saveAs(blob, 'recovery_codes.txt');
            }
          }}
        />
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconCopy />}
          text="Copy"
          onClick={() => {
            if (codes) {
              clipboard
                .write(codes.join('\n'))
                .then(() => {
                  toaster.success('Codes copied');
                })
                .catch((err) => {
                  console.error(err);
                  toaster.error('Clipboard unaccessible');
                });
            }
          }}
        />
      </div>
      <div className="controls">
        <Button
          text="I have saved my codes."
          onClick={() => mutate()}
          styleVariant={ButtonStyleVariant.WARNING}
          size={ButtonSize.BIG}
          loading={isLoading}
        />
      </div>
    </>
  );
};
