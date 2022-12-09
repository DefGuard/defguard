import './style.scss';

import classNames from 'classnames';
import { useMemo } from 'react';

import { IconHamburgerClose } from '../../svg';
import Modal, { ModalProps } from '../Modal/Modal';

export interface ModalWithTitleProps extends ModalProps {
  title: string;
}

export const ModalWithTitle = ({
  children,
  title,
  className,
  isOpen,
  setIsOpen,
  disableClose = false,
  ...rest
}: ModalWithTitleProps) => {
  const cn = useMemo(() => classNames('titled', className), [className]);
  return (
    <Modal
      className={cn}
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      disableClose={disableClose}
      {...rest}
    >
      <div className="header">
        <p className="title">{title}</p>
        {!disableClose && (
          <button
            className="close"
            onClick={() => {
              setIsOpen(false);
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
