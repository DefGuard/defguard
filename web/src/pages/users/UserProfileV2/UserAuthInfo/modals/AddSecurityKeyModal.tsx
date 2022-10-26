import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import { ModalWithTitle } from '../../../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';

export const AddSecuritykeyModal = () => {
  const modalState = useModalStore((state) => state.addSecurityKeyModal);
  const setModalState = useModalStore((state) => state.setState);
  return (
    <ModalWithTitle
      title="Add security key"
      isOpen={modalState.visible}
      setIsOpen={(visibility) =>
        setModalState({ addSecurityKeyModal: { visible: visibility } })
      }
    ></ModalWithTitle>
  );
};

const AddSecurityKeyForm = () => {
  const setModalState = useModalStore((state) => state.setState);
  return (
    <form>
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          className="cancel"
          type="button"
          size={ButtonSize.BIG}
          text="Cancel"
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          size={ButtonSize.BIG}
          type="submit"
          text="Confirm code"
        />
      </div>
    </form>
  );
};
