import './style.scss';

import { clone } from 'lodash-es';
import React, { ReactNode, useMemo } from 'react';

import Button, { ButtonSize, ButtonStyleVariant } from '../Button/Button';
import Modal from '../Modal/Modal';

export enum ConfirmModalType {
  NORMAL = 'NORMAL',
  WARNING = 'WARNING',
}

interface Props {
  isOpen: boolean;
  setIsOpen: (v: boolean) => void;
  onSubmit: () => void;
  title: string;
  submitText: string;
  type?: ConfirmModalType;
  subTitle?: string | ReactNode;
  cancelText?: string;
  loading?: boolean;
}

const baseClass = 'modal middle confirm';

/**
 * Reusable modal configuration for modals confirming an action.
 * @param isOpen Visibility state, passed to `Modal` component.
 * @param setIsOpen Setter for `isOpen`
 * @param type Style variant
 * @param title Displayed title in modal
 * @param loading Passed into `Button` loading param
 * @param cancelText Text inside of cancel button
 * @param submitText Text inside of submit / confirmation button
 * @param subTitle Optional text under modal's title
 */
const ConfirmModal: React.FC<Props> = ({
  isOpen,
  setIsOpen,
  type,
  title,
  loading,
  cancelText,
  submitText,
  onSubmit,
  subTitle,
}) => {
  const getMainClass = useMemo(() => {
    let res = clone(baseClass);
    if (type === ConfirmModalType.WARNING) {
      res = res + ' warning';
    }
    return res;
  }, [type]);

  return (
    <Modal
      backdrop
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      className={getMainClass}
    >
      <p className="title">{title}</p>
      <p className="subtitle">{subTitle}</p>
      <section className="controls">
        <Button
          size={ButtonSize.BIG}
          className="cancel"
          text={cancelText ?? 'Cancel'}
          onClick={() => setIsOpen(false)}
        />
        <Button
          size={ButtonSize.BIG}
          styleVariant={
            type === ConfirmModalType.WARNING
              ? ButtonStyleVariant.CONFIRM_WARNING
              : ButtonStyleVariant.WARNING
          }
          disabled={loading}
          loading={loading}
          onClick={onSubmit}
          text={submitText}
        />
      </section>
    </Modal>
  );
};

export default ConfirmModal;
