import './style.scss';

import classNames from 'classnames';
import { isUndefined } from 'lodash-es';
import { ComponentPropsWithoutRef, ReactNode, useEffect, useMemo, useState } from 'react';

import SvgIconInfo from '../../svg/IconInfo';
import SvgIconInfoError from '../../svg/IconInfoError';
import SvgIconInfoSuccess from '../../svg/IconInfoSuccess';
import SvgIconInfoWarning from '../../svg/IconInfoWarning';
import SvgIconX from '../../svg/IconX';
import { MessageBoxType } from './types';
import { readMessageBoxVisibility, writeMessageBoxVisibility } from './utils';

interface Props extends ComponentPropsWithoutRef<'div'> {
  message?: string | ReactNode;
  type?: MessageBoxType;
  dismissId?: string;
}

/**
 * Styled box with message.
 */
export const MessageBox = ({
  message,
  className,
  dismissId,
  type = MessageBoxType.INFO,
  ...props
}: Props) => {
  const [visible, setVisible] = useState(true);

  const dismissable = !isUndefined(dismissId);

  const getClassName = useMemo(() => {
    return classNames('message-box', className, type.valueOf());
  }, [className, type]);

  const getIcon = useMemo(() => {
    switch (type) {
      case MessageBoxType.INFO:
        return <SvgIconInfo />;
      case MessageBoxType.SUCCESS:
        return <SvgIconInfoSuccess />;
      case MessageBoxType.WARNING:
        return <SvgIconInfoWarning />;
      case MessageBoxType.ERROR:
        return <SvgIconInfoError />;
    }
  }, [type]);

  const renderMessage = useMemo(() => {
    if (typeof message === 'string') {
      return <p>{message}</p>;
    }
    return message;
  }, [message]);

  useEffect(() => {
    if (dismissId && dismissId.length) {
      const visibility = readMessageBoxVisibility(dismissId);
      if (visible !== visibility) {
        setVisible(visibility);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!visible) return null;

  return (
    <div className={getClassName} {...props}>
      <div className="icon-container">{getIcon}</div>
      <div className="message">{renderMessage}</div>
      {dismissable && (
        <button
          className="dismiss"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            writeMessageBoxVisibility(dismissId);
            setVisible(false);
          }}
        >
          <SvgIconX />
        </button>
      )}
    </div>
  );
};
