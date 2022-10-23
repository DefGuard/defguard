import './style.scss';

import { useMemo } from 'react';
import useBreakpoint from 'use-breakpoint';

import { deviceBreakpoints } from '../../../constants';
import { IconHamburgerClose } from '../../svg';
import Button, { ButtonSize, ButtonStyleVariant } from '../Button/Button';
import IconButton from '../IconButton/IconButton';
import Modal, { ModalProps } from '../Modal/Modal';

interface Props extends Omit<ModalProps, 'backdrop'> {
  side: 'left' | 'right';
  header: string | React.ReactNode;
}

export const SideFormModal = ({
  isOpen,
  setIsOpen,
  children,
  className,
  side,
  header,
}: Props) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const getClassName = useMemo(() => {
    const res = ['side-form', side];
    if (className) {
      res.push(className);
    }
    return res.join(' ');
  }, [className, side]);

  return (
    <Modal
      backdrop
      className={getClassName}
      isOpen={isOpen}
      setIsOpen={setIsOpen}
    >
      <header>
        <h3>{header}</h3>
        {breakpoint !== 'desktop' && (
          <IconButton
            className="blank"
            whileHover={{ scale: 1.2 }}
            onClick={() => setIsOpen(false)}
          >
            <IconHamburgerClose />
          </IconButton>
        )}
      </header>
      {children}
      {breakpoint === 'desktop' && (
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          className="close"
          onClick={() => setIsOpen(false)}
          text="Cancel"
        />
      )}
    </Modal>
  );
};
