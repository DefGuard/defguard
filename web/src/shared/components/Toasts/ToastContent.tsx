import './style.scss';

import React, { useMemo } from 'react';

import SvgIconInfoError from '../svg/IconInfoError';
import SvgIconInfoNormal from '../svg/IconInfoNormal';
import SvgIconInfoSuccess from '../svg/IconInfoSuccess';
import SvgIconInfoWarning from '../svg/IconInfoWarning';

export enum ToastType {
  INFO = 'info',
  WARNING = 'warning',
  SUCCESS = 'success',
  ERROR = 'error',
}

interface ToastContentProps {
  type: ToastType;
  message: string;
  subMessage?: string;
  progress?: boolean;
}

/**
 * Replaces default body of toasts from react-toastify. Should be passed as a first argument to `toast` function.
 * @param type Style variant
 * @param message main text of toast
 * @param subMessage Currently only supported by type `INFO`, text appear under main text
 * @param progress Information if toast has enabled progress.
 */
const ToastContent: React.FC<ToastContentProps> = ({
  type,
  message,
  subMessage,
  progress = false,
}) => {
  const getClass = useMemo(() => {
    const res = ['toast'];
    res.push(type.valueOf());
    if (progress) {
      res.push('progress');
    }
    return res.join(' ');
  }, [type, progress]);

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
      <p>
        {message}
        {subMessage && subMessage.length ? <span>{subMessage}</span> : null}
      </p>
    </div>
  );
};

export default ToastContent;
