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
  ...rest
}: ModalWithTitleProps) => {
  const cn = useMemo(() => classNames('titled', className), [className]);
  return (
    <Modal className={cn} {...rest}>
      <div className="header">
        <p className="title">{title}</p>
        <button className="close">
          <IconHamburgerClose />
        </button>
      </div>
      <div className="content">{children}</div>
    </Modal>
  );
};
