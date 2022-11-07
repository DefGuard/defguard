import './style.scss';

import classNames from 'classnames';
import { motion, Variants } from 'framer-motion';
import React, { ComponentPropsWithoutRef, ReactNode, useMemo } from 'react';

import { ColorsRGB } from '../../../constants';

export interface CheckBoxProps extends ComponentPropsWithoutRef<'input'> {
  label?: string | ReactNode;
  value: string | number;
  labelPosition?: 'left' | 'right';
  onChange?: (value: unknown) => void;
}

export const CheckBox = React.forwardRef<HTMLInputElement, CheckBoxProps>(
  ({ label, value, onChange, labelPosition = 'right', ...props }, ref) => {
    const checked = useMemo(() => (Number(value) ? true : false), [value]);

    const cn = useMemo(
      () =>
        classNames('custom-checkbox', {
          checked: checked,
          'label-left': labelPosition === 'left',
          'label-right': labelPosition === 'right',
        }),
      [checked, labelPosition]
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
        <motion.div
          className="box"
          variants={boxVariants}
          animate={checked ? 'boxChecked' : 'boxDefault'}
        >
          <motion.div
            className="check-box"
            variants={checkBoxVariants}
          ></motion.div>
        </motion.div>
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

const boxVariants: Variants = {
  boxDefault: {},
  boxChecked: {},
};

const checkBoxVariants: Variants = {
  boxDefault: {
    height: 17,
    width: 17,
    borderRadius: 6,
    borderColor: ColorsRGB.GrayBorder,
    backgroundColor: ColorsRGB.BgLight,
  },
  boxChecked: {
    height: 7,
    width: 7,
    borderRadius: 2,
    borderColor: ColorsRGB.Transparent,
    backgroundColor: ColorsRGB.White,
  },
};
