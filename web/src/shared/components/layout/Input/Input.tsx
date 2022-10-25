import './style.scss';

import {
  AnimatePresence,
  HTMLMotionProps,
  motion,
  Variant,
  Variants,
} from 'framer-motion';
import { isUndefined } from 'lodash-es';
import React, { useId, useMemo, useRef } from 'react';
import { useState } from 'react';

import {
  buttonsBoxShadow,
  ColorsRGB,
  inactiveBoxShadow,
} from '../../../constants';
import SvgIconAsterix from '../../svg/IconAsterix';
import SvgIconCheckmarkGreen from '../../svg/IconCheckmarkGreen';
import SvgIconPopupClose from '../../svg/IconPopupClose';
import SvgIconWarning from '../../svg/IconWarning';

export interface InputProps extends HTMLMotionProps<'input'> {
  required?: boolean;
  disposable?: boolean;
  disposeHandler?: (v?: unknown) => void;
  invalid?: boolean;
  validate?: (v?: unknown) => void;
  valid?: boolean;
  innerLabel?: boolean;
  outerLabel?: string;
  disableOuterLabelColon?: boolean;
  errorMessage?: string;
}

interface PlaceholderCustom {
  focused: boolean;
  innerLabel: boolean;
}

interface ContainerCustom {
  hovered: boolean;
  invalid: boolean;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      required,
      invalid,
      valid,
      value,
      disposable,
      disposeHandler,
      placeholder,
      innerLabel = false,
      errorMessage,
      outerLabel,
      disableOuterLabelColon,
      ...props
    },
    forwardedRef
  ) => {
    const innerInputRef = useRef<HTMLInputElement | null>(null);
    const [focused, setFocused] = useState(false);
    const [hovered, setHovered] = useState(false);
    const componentId = useId();
    const getClassName = useMemo(() => {
      const res = ['input-container'];
      if (invalid) {
        res.push('invalid');
      }
      if (disposable) {
        res.push('disposable');
      }
      if (innerLabel) {
        res.push('inner-label');
      }
      if (!isUndefined(errorMessage)) {
        res.push('has-error-message');
      }
      return res.join(' ');
    }, [disposable, errorMessage, innerLabel, invalid]);

    const getIconProp = useMemo(() => {
      if (required && !invalid && !valid) {
        return 'asterix';
      }
      if (invalid) {
        return 'invalid';
      }
      if (valid) {
        return 'valid';
      }
      return 'idle';
    }, [required, invalid, valid]);

    const getPlaceholderCustom: PlaceholderCustom = useMemo(
      () => ({ focused, innerLabel }),
      [focused, innerLabel]
    );

    const getContainerCustom: ContainerCustom = useMemo(
      () => ({ invalid: Boolean(invalid), hovered }),
      [invalid, hovered]
    );

    return (
      <>
        {outerLabel && outerLabel.length > 0 && (
          <label className="input-outer-label" htmlFor={componentId}>
            {outerLabel}
            {!disableOuterLabelColon && ':'}
          </label>
        )}
        <motion.div
          className={getClassName}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          onHoverStart={() => setHovered(true)}
          onHoverEnd={() => setHovered(false)}
          onClick={() => {
            if (innerInputRef) {
              innerInputRef.current?.focus();
            }
          }}
          variants={containerVariants}
          initial="idle"
          animate={focused ? 'focused' : 'idle'}
          custom={getContainerCustom}
        >
          <motion.input
            variants={inputVariants}
            initial="idle"
            animate={focused ? 'focused' : 'idle'}
            custom={getContainerCustom}
            ref={(r) => {
              innerInputRef.current = r;
              if (typeof forwardedRef === 'function') {
                forwardedRef(r);
              } else {
                if (forwardedRef) {
                  forwardedRef.current = r;
                }
              }
            }}
            {...props}
            value={value}
            placeholder={placeholder}
            id={componentId}
          />
          <AnimatePresence mode="wait">
            <motion.span
              className="icon-container"
              variants={iconContainerVariants}
              initial="hidden"
              animate={getIconProp}
              exit="hidden"
              key={getIconProp}
            >
              {required && !invalid && !valid ? <SvgIconAsterix /> : null}
              {invalid ? <SvgIconWarning /> : null}
              {valid ? <SvgIconCheckmarkGreen /> : null}
            </motion.span>
          </AnimatePresence>
          {disposable && focused ? (
            <div className="dispose" onClick={disposeHandler}>
              <div>
                <SvgIconPopupClose />
              </div>
            </div>
          ) : null}
          {placeholder && !value?.toString().length ? (
            <motion.span
              className="placeholder-label"
              variants={placeholderVariants}
              initial={false}
              animate={focused && value?.toString().length ? 'focused' : 'idle'}
              custom={getPlaceholderCustom}
            >
              {placeholder}
            </motion.span>
          ) : null}
          <AnimatePresence>
            {invalid && errorMessage ? (
              <motion.span
                className="error-message"
                initial={{
                  x: 20,
                  opacity: 0,
                  bottom: 0,
                }}
                animate={{
                  x: 20,
                  opacity: 1,
                  bottom: -20,
                }}
                exit={{
                  opacity: 0,
                  x: 20,
                  bottom: -20,
                }}
              >
                {errorMessage}
              </motion.span>
            ) : null}
          </AnimatePresence>
        </motion.div>
      </>
    );
  }
);

const containerVariants: Variants = {
  idle: ({ hovered }: ContainerCustom) => {
    let res: Variant = {
      boxShadow: inactiveBoxShadow,
      backgroundColor: ColorsRGB.White,
    };
    if (hovered) {
      res = {
        ...res,
        boxShadow: buttonsBoxShadow,
      };
    }
    return res;
  },
  focused: ({ invalid }: ContainerCustom) => {
    let res = {
      boxShadow: buttonsBoxShadow,
    };
    if (invalid) {
      res = { ...res, boxShadow: buttonsBoxShadow };
    }
    return res;
  },
};

const inputVariants: Variants = {
  idle: ({ hovered, invalid }: ContainerCustom) => {
    let res: Variant = {
      borderColor: ColorsRGB.GrayBorder,
    };
    if (hovered) {
      res = {
        ...res,
        borderColor: ColorsRGB.GrayLighter,
      };
    }
    if (invalid) {
      res = { ...res, borderColor: ColorsRGB.Error };
    }
    return res;
  },
  focused: ({ invalid }: ContainerCustom) => {
    let res = {
      borderColor: ColorsRGB.GrayLighter,
    };
    if (invalid) {
      res = { ...res, borderColor: ColorsRGB.Error };
    }
    return res;
  },
};

const placeholderVariants: Variants = {
  idle: {
    fontSize: '15px',
    lineHeight: '1.4rem',
    color: ColorsRGB.GrayLight,
    x: 20,
    y: 18,
  },
  focused: ({ innerLabel }: PlaceholderCustom) => {
    if (!innerLabel) {
      return {
        opacity: 0,
        fontSize: '15px',
        lineHeight: '1.4rem',
        color: ColorsRGB.GrayLight,
        x: 20,
        y: 18,
      };
    }
    return {
      x: 20,
      y: 4,
      fontSize: '12px',
      lineHeight: '1.4rem',
      color: ColorsRGB.GrayLight,
    };
  },
};

const iconContainerVariants: Variants = {
  asterix: {
    opacity: 1,
    width: '22px',
    height: '22px',
  },
  valid: {
    opacity: 1,
    width: '22px',
    height: '22px',
  },
  invalid: {
    opacity: 1,
    width: '18px',
    height: '18px',
  },
  idle: {
    opacity: 1,
    width: '22px',
    height: '22px',
  },
  hidden: {
    opacity: 0,
  },
};
