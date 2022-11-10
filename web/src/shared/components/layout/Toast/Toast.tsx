import './style.scss';

import { useMemo } from 'react';

import SvgIconInfoError from '../../svg/IconInfoError';
import SvgIconInfoNormal from '../../svg/IconInfoNormal';
import SvgIconInfoSuccess from '../../svg/IconInfoSuccess';
import SvgIconInfoWarning from '../../svg/IconInfoWarning';
import classNames from 'classnames';
import Button, { ButtonStyleVariant } from '../Button/Button';
import { IconPopupClose } from '../../svg';

export enum ToastType {
  INFO = 'info',
  WARNING = 'warning',
  SUCCESS = 'success',
  ERROR = 'error',
}

export interface CustomToastContentProps {
  type: ToastType;
  message: string;
  subMessage?: string;
  progress?: boolean;
}


export const Toast = ({
  type,
  message,
  subMessage,
}: CustomToastContentProps) => {
  const getClass = useMemo(() => classNames('toast', type.valueOf()), [type]);

  const getIcon = useMemo(() => {
    if (type === ToastType.INFO && !subMessage) {
      return <SvgIconInfoNormal />;
    }
    if (type === ToastType.ERROR) {
      return <SvgIconInfoError />;
    }
    if (type === ToastType.WARNING) {
      return <SvgIconInfoWarning />;
    }
    if (type === ToastType.SUCCESS) {
      return <SvgIconInfoSuccess />;
    }
    return null;
  }, [type, subMessage]);

  return (
    <div className={getClass}>
      {getIcon}
      <p className='message'>
        {message}
      </p>
      {subMessage && subMessage.length && (
        <p className="sub-message">{subMessage}</p>
      )}
      <Button icon={<IconPopupClose />} styleVariant={ButtonStyleVariant.ICON} />
    </div>
  );
};