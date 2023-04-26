import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

import { ColorsRGB } from '../../../constants';

export interface CheckBoxProps extends Omit<HTMLMotionProps<'div'>, 'onChange'> {
  label?: string | ReactNode;
  value: boolean;
  labelPosition?: 'left' | 'right';
  disabled?: boolean;
  onChange?: (value: boolean) => void;
}

export const CheckBox = ({
  label,
  value,
  onChange,
  labelPosition = 'right',
  disabled = false,
  ...rest
}: CheckBoxProps) => {
  const checked = useMemo(() => (Number(value) ? true : false), [value]);

  const cn = useMemo(
    () =>
      classNames('checkbox', {
        checked: checked,
        'label-left': labelPosition === 'left',
        'label-right': labelPosition === 'right',
        disabled: disabled,
      }),
    [checked, disabled, labelPosition]
  );

  return (
    <motion.div
      {...rest}
      className={cn}
      onClick={() => {
        if (onChange && !disabled) {
          onChange(!value);
        }
      }}
      variants={containerVariants}
      animate={disabled ? 'containerDisabled' : 'containerIdle'}
    >
      <motion.div className="box">
        <motion.div
          className="check-box"
          variants={checkBoxVariants}
          animate={checked ? 'boxChecked' : 'boxDefault'}
        ></motion.div>
      </motion.div>
      {label ? <div className="label">{label}</div> : null}
    </motion.div>
  );
};

const containerVariants: Variants = {
  containerIdle: { opacity: 1 },
  containerDisabled: { opacity: 0.8 },
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
