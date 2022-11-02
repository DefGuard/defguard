import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../../shared/components/layout/MessageBox/MessageBox';
import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { IconCopy, IconDownload } from '../../../../../shared/components/svg';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';

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

  if (!codes) return null;
  return (
    <>
      <MessageBox type={MessageBoxType.INFO}>
        <p>
          Treat your recovery codes with the same level of attention as you
          would your password! We recommend saving them with a password manager
          such as Lastpass, 1Password, or Keeper.
        </p>
      </MessageBox>
      <ul className="codes">
        {codes.map((code) => (
          <li key="code">{code}</li>
        ))}
      </ul>
      <div className="actions">
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconDownload />}
        />
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconCopy />}
        />
      </div>
      <div className="controls">
        <Button
          className="cancel"
          text="Cancel"
          onClick={() => setModalState({ visible: false, codes: undefined })}
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.BIG}
        />
      </div>
    </>
  );
};
