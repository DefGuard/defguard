import './style.scss';

import classNames from 'classnames';
import React, { ComponentPropsWithoutRef, useMemo } from 'react';

import SvgIconDeactivated from '../../svg/IconDeactivated';
import SvgIconInfo from '../../svg/IconInfo';
import SvgIconInfoError from '../../svg/IconInfoError';
import SvgIconInfoSuccess from '../../svg/IconInfoSuccess';
import SvgIconInfoWarning from '../../svg/IconInfoWarning';

export enum MessageBoxType {
  INFO = 'info',
  WARNING = 'warning',
  SUCCESS = 'success',
  DEACTIVATED = 'deactivated',
  ERROR = 'error',
}

interface Props extends ComponentPropsWithoutRef<'div'> {
  message?: string;
  type?: MessageBoxType;
}

/**
 * Styled box with message.
 * children should be only one container element.
 */
const MessageBox = ({
  children,
  message,
  type = MessageBoxType.INFO,
  ...props
}: Props) => {
  const getClassName = useMemo(() => {
    return classNames('message-box', props.className, type.valueOf());
  }, [props.className, type]);

  const getIcon = useMemo(() => {
    switch (type) {
      case MessageBoxType.INFO:
        return <SvgIconInfo />;
      case MessageBoxType.SUCCESS:
        return <SvgIconInfoSuccess />;
      case MessageBoxType.WARNING:
        return <SvgIconInfoWarning />;
      case MessageBoxType.DEACTIVATED:
        return <SvgIconDeactivated />;
      case MessageBoxType.ERROR:
        return <SvgIconInfoError />;
    }
  }, [type]);

  return (
    <div className="message-box-container">
      <div {...props} className={getClassName}>
        {getIcon}
        {message && !children ? <p>{message}</p> : null}
        {children}
      </div>
    </div>
  );
};

export default MessageBox;
