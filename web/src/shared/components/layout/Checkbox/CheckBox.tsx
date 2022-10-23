import './style.scss';

import classNames from 'classnames';
import React, { ComponentPropsWithoutRef, useMemo } from 'react';

export interface CheckBoxProps extends ComponentPropsWithoutRef<'input'> {
  label?: string;
  value: string | number;
  onChange?: (value: unknown) => void;
}

export const CheckBox = React.forwardRef<HTMLInputElement, CheckBoxProps>(
  ({ label, value, onChange, ...props }, ref) => {
    const checked = useMemo(() => (Number(value) ? true : false), [value]);

    const cn = useMemo(
      () => classNames('custom-checkbox', { checked: checked }),
      [checked]
    );

    return (
      <div
        className={cn}
        onClick={() => {
          if (onChange) {
            onChange(!value);
          }
        }}
      >
        <div className="box"></div>
        {label ? <label>{label}</label> : null}
        <input
          ref={ref}
          type="checkbox"
          checked={checked}
          onChange={onChange}
          {...props}
        />
      </div>
    );
  }
);
