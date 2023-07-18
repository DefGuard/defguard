import './style.scss';

import classNames from 'classnames';
import { useMemo } from 'react';

import { IconHamburgerClose } from '../../svg';
import { Modal } from '../Modal/Modal';
import { ModalProps } from '../Modal/types';

export interface ModalWithTitleProps extends ModalProps {
  title: string;
}

export const ModalWithTitle = ({
  children,
  title,
  className,
  isOpen,
  onClose,
  setIsOpen,
  disableClose = false,
  ...rest
}: ModalWithTitleProps) => {
  const cn = useMemo(() => classNames('titled', className), [className]);
  return (
    <Modal
      onClose={onClose}
      setIsOpen={setIsOpen}
      className={cn}
      isOpen={isOpen}
      disableClose={disableClose}
      {...rest}
    >
      <div className="header">
        <p className="title">{title}</p>
        {!disableClose && (
          <button
            className="close"
            onClick={() => {
              onClose && onClose();
              setIsOpen && setIsOpen(false);
            }}
          >
            <IconHamburgerClose />
          </button>
        )}
      </div>
      {children && <div className="content">{children}</div>}
    </Modal>
  );
};
