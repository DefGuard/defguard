import './style.scss';

import classNames from 'classnames';
import { isUndefined } from 'lodash-es';
import React, { ButtonHTMLAttributes, useMemo, useState } from 'react';

import { LoaderSpinner } from '../LoaderSpinner/LoaderSpinner';
import { ButtonSize, ButtonStyleVariant } from './types';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  loading?: boolean;
  size?: ButtonSize;
  styleVariant?: ButtonStyleVariant;
  text?: string;
  icon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Button = ({
  loading = false,
  size = ButtonSize.SMALL,
  styleVariant = ButtonStyleVariant.STANDARD,
  text,
  icon,
  rightIcon,
  className,
  disabled = false,
  type = 'button',
  onClick,
  ...props
}: Props) => {
  const [hovered, setHovered] = useState(false);
  const isDisabled = useMemo(() => disabled || loading, [disabled, loading]);

  const getClassName = useMemo(
    () =>
      classNames('btn', className, size.valueOf(), styleVariant.valueOf(), {
        icon: !isUndefined(icon),
        loading: loading,
        hovered: hovered && !loading && !disabled,
        disabled,
      }),
    [className, size, styleVariant, icon, loading, hovered, disabled],
  );

  const getButtonStyle = useMemo((): Props['style'] => {
    const textColumn = 'min-content';
    const res: Props['style'] = {
      gridTemplateColumns: textColumn,
    };
    const columnSize = size === ButtonSize.LARGE ? `36px` : `18px`;
    if (!text) {
      if (icon && (rightIcon || loading)) {
        res.gridTemplateColumns = `${columnSize} ${columnSize}`;
        return res;
      } else {
        res.gridTemplateColumns = columnSize;
        return res;
      }
    }
    if (text && !icon && !rightIcon && !loading) {
      res.gridTemplateColumns = textColumn;
      return res;
    }
    if (text && icon && !loading && !rightIcon) {
      res.gridTemplateColumns = `${columnSize} ${textColumn}`;
      return res;
    }
    if (text && (loading || rightIcon) && !icon) {
      res.gridTemplateColumns = `${textColumn} ${columnSize}`;
      return res;
    }
    if (icon && text && (loading || rightIcon)) {
      res.gridTemplateColumns = `${columnSize} ${textColumn} ${columnSize}`;
      return res;
    }
    return res;
  }, [icon, loading, rightIcon, size, text]);

  return (
    <button
      style={getButtonStyle}
      type={type}
      className={getClassName}
      disabled={isDisabled}
      onClick={(e) => {
        if (!disabled && !loading && (onClick || type != 'button')) {
          if (onClick) {
            onClick(e);
          }
        } else {
          e.preventDefault();
          e.stopPropagation();
        }
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      {...props}
    >
      {icon}
      {text && text.length > 0 && <span className="text">{text}</span>}
      {rightIcon && !loading && <>{rightIcon}</>}
      {loading && <LoaderSpinner size={size === ButtonSize.LARGE ? 26 : 12} />}
    </button>
  );
};
