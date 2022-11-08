import './style.scss';

import SvgIconHamburgerClose from '../../svg/IconHamburgerClose';
import IconButton from '../IconButton/IconButton';
import Modal from '../Modal/Modal';

interface Props {
  title: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  FormComponent: React.FC<any>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  formComponentProps?: Record<string, unknown>;
  isOpen: boolean;
  setIsOpen: (v: boolean) => void;
  smallTopMargin?: boolean;
}

/**
 * @depraceted Part of old design. Use {@link ModalWithTitle} instead.
 */
const MiddleFormModal: React.FC<Props> = ({
  title,
  FormComponent,
  formComponentProps = {},
  isOpen,
  setIsOpen,
  smallTopMargin = false,
}: Props) => {
  return (
    <Modal
      setIsOpen={setIsOpen}
      isOpen={isOpen}
      className={`form middle ${smallTopMargin ? 'top-margin-small' : ''}`}
      backdrop
    >
      <header>
        <p className="title">{title}</p>
        <IconButton className="blank" onClick={() => setIsOpen(false)}>
          <SvgIconHamburgerClose />
        </IconButton>
      </header>
      <div className="form-container">
        <FormComponent setIsOpen={setIsOpen} {...formComponentProps} />
      </div>
    </Modal>
  );
};

export default MiddleFormModal;
