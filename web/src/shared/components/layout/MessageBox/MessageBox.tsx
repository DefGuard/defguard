import './style.scss';

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

interface Props {
  message?: string;
  type?: MessageBoxType;
}

/**
 * Styled box with message.
 * @param message
 * Displayed string in the box.
 * @param type
 * Determinate styling and icon of a box.
 */
const MessageBox: React.FC<ComponentPropsWithoutRef<'div'> & Props> = ({
  children,
  message,
  type = MessageBoxType.INFO,
  ...props
}) => {
  const getClassName = useMemo(() => {
    const res = ['message-box'];
    res.push(type.valueOf());
    if (props.className && props.className.length) {
      const definedClasses = props.className.split(' ');
      res.push(...definedClasses);
    }
    return res.join(' ');
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
    <div {...props} className={getClassName}>
      {getIcon}
      {message ? <p>{message}</p> : null}
      {children}
    </div>
  );
};

export default MessageBox;
