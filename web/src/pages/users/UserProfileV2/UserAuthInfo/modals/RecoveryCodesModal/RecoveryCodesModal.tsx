import './style.scss';

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
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { useToaster } from '../../../../../../shared/hooks/useToaster';


export const RecoveryCodesModal = () => {
  const modalState = useModalStore((state) => state.recoveryCodesModal);
  const setModalState = useModalStore((state) => state.setRecoveryCodesModal);

  return (
    <ModalWithTitle
      id="view-recovery-codes"
      title="Recovery codes"
      isOpen={modalState.visible}
      setIsOpen={(visible) => setModalState({ visible, codes: undefined })}
      backdrop
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const codes = useModalStore((state) => state.recoveryCodesModal.codes);
  const setModalState = useModalStore((state) => state.setRecoveryCodesModal);
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
              const blob = new Blob(codes, {
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
                  toaster.error('Clipboard unaccessable');
                });
            }
          }}
        />
      </div>
      <div className="controls">
        <Button
          className="cancel"
          text="Close"
          onClick={() => setModalState({ visible: false, codes: undefined })}
          styleVariant={ButtonStyleVariant.WARNING}
          size={ButtonSize.BIG}
        />
      </div>
    </>
  );
};
