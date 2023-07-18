import './style.scss';

import {
  arrow,
  autoUpdate,
  flip,
  FloatingPortal,
  offset,
  useFloating,
} from '@floating-ui/react-dom-interactions';
import classNames from 'classnames';
import {
  AnimatePresence,
  HTMLMotionProps,
  motion,
  Variant,
  Variants,
} from 'framer-motion';
import { isUndefined } from 'lodash-es';
import React, { ReactNode, useEffect, useId, useMemo, useRef, useState } from 'react';

import { buttonsBoxShadow, ColorsRGB, inactiveBoxShadow } from '../../../constants';
import SvgIconAsterix from '../../svg/IconAsterix';
import SvgIconCheckmarkGreen from '../../svg/IconCheckmarkGreen';
import SvgIconPopupClose from '../../svg/IconPopupClose';
import SvgIconWarning from '../../svg/IconWarning';
import { FloatingArrow } from '../FloatingArrow/FloatingArrow';
import { FloatingBox } from '../FloatingBox/FloatingBox';

export type InputFloatingErrors = {
  title: string;
  errorMessages: string[];
};

export interface InputProps extends HTMLMotionProps<'input'> {
  required?: boolean;
  disposable?: boolean;
  disposeHandler?: (v?: unknown) => void;
  invalid?: boolean;
  validate?: (v?: unknown) => void;
  valid?: boolean;
  innerLabel?: boolean;
  outerLabel?: string | ReactNode;
  disableOuterLabelColon?: boolean;
  errorMessage?: string;
  floatingErrors?: InputFloatingErrors;
}

interface PlaceholderCustom {
  focused: boolean;
  innerLabel: boolean;
}

interface ContainerCustom {
  hovered: boolean;
  invalid: boolean;
  disabled: boolean;
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
      disabled = false,
      errorMessage,
      outerLabel,
      disableOuterLabelColon,
      floatingErrors,
      ...props
    },
    forwardedRef,
  ) => {
    const innerInputRef = useRef<HTMLInputElement | null>(null);
    const [focused, setFocused] = useState(false);
    const [hovered, setHovered] = useState(false);
    const componentId = useId();
    const [floatingErrorsOpen, setFloatingErrorsOpen] = useState(false);
    const floatingErrorsArrow = useRef<HTMLDivElement | null>(null);
    const { reference, floating, x, y, strategy, placement, middlewareData } =
      useFloating({
        open: floatingErrorsOpen,
        onOpenChange: setFloatingErrorsOpen,
        placement: 'bottom-end',
        middleware: [
          offset(10),
          flip(),
          arrow({
            element: floatingErrorsArrow,
          }),
        ],
        whileElementsMounted: (refElement, floatingElement, updateFunc) =>
          autoUpdate(refElement, floatingElement, updateFunc),
      });

    const getClassName = useMemo(() => {
      const hasErrorMessage = !isUndefined(errorMessage);
      return classNames('input-container', {
        invalid,
        disposable: disposable && !disabled,
        'inner-label': innerLabel,
        'has-error-message': hasErrorMessage && !disabled,
        disabled,
      });
    }, [disabled, disposable, errorMessage, innerLabel, invalid]);

    const getOuterLabelClassName = useMemo(
      () =>
        classNames('input-outer-label', {
          disabled,
        }),
      [disabled],
    );

    const getContainerVariant = useMemo(() => {
      if (disabled) {
        return 'idle';
      }
      if (hovered || focused) {
        return 'focused';
      }
      return 'idle';
    }, [disabled, focused, hovered]);

    const getPlaceholderCustom: PlaceholderCustom = useMemo(
      () => ({ focused, innerLabel, disabled }),
      [disabled, focused, innerLabel],
    );

    const getContainerCustom: ContainerCustom = useMemo(
      () => ({ invalid: Boolean(invalid), disabled, hovered }),
      [invalid, disabled, hovered],
    );

    const getInputIcon: ReactNode = useMemo(() => {
      if (disabled) {
        return null;
      }
      if (invalid) {
        return <SvgIconWarning />;
      }
      if (valid) {
        return <SvgIconCheckmarkGreen />;
      }
      if (required) {
        return <SvgIconAsterix />;
      }
      return null;
    }, [disabled, invalid, required, valid]);

    // control floatingErrors while typing
    useEffect(() => {
      if (
        focused &&
        floatingErrors &&
        floatingErrors.errorMessages.length > 0 &&
        !floatingErrorsOpen
      ) {
        setFloatingErrorsOpen(true);
      }
      if (
        (!focused && floatingErrorsOpen) ||
        (floatingErrorsOpen &&
          floatingErrors &&
          floatingErrors.errorMessages.length === 0)
      ) {
        setFloatingErrorsOpen(false);
      }
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [floatingErrors, floatingErrorsOpen]);

    return (
      <>
        {outerLabel && (
          <motion.label
            className={getOuterLabelClassName}
            htmlFor={componentId}
            variants={outerLabelVariants}
            custom={getContainerCustom}
            initial={false}
            animate="idle"
            onClick={() => {
              if (innerInputRef) {
                innerInputRef.current?.focus();
              }
            }}
          >
            {outerLabel}
            {!disableOuterLabelColon && ':'}
          </motion.label>
        )}
        <motion.div
          ref={reference}
          className={getClassName}
          onFocus={() => {
            setFocused(true);
            if (floatingErrors && floatingErrors.errorMessages.length) {
              setFloatingErrorsOpen(true);
            }
          }}
          onBlur={() => {
            setFocused(false);
            if (floatingErrorsOpen) {
              setFloatingErrorsOpen(false);
            }
          }}
          onHoverStart={() => setHovered(true)}
          onHoverEnd={() => setHovered(false)}
          onClick={() => {
            if (innerInputRef) {
              innerInputRef.current?.focus();
            }
          }}
          variants={containerVariants}
          initial="idle"
          animate={getContainerVariant}
          custom={getContainerCustom}
        >
          <motion.input
            variants={inputVariants}
            initial="idle"
            animate={getContainerVariant}
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
            disabled={disabled}
          />
          <span className="icon-container">{getInputIcon}</span>
          {!disabled && disposable && hovered ? (
            <div
              className="dispose"
              onClick={() => {
                if (disposeHandler) {
                  disposeHandler();
                }
              }}
            >
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
            {invalid && errorMessage && !disabled ? (
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
        <FloatingPortal>
          {floatingErrorsOpen && floatingErrors && (
            <FloatingBox
              className="floating-input-errors"
              ref={floating}
              style={{
                position: strategy,
                left: x || 0,
                top: y || 0,
              }}
            >
              <p className="title">{floatingErrors.title}</p>
              <div className="errors">
                {floatingErrors.errorMessages.map((errorMessage) => (
                  <p className="error" key={errorMessage}>
                    <SvgIconWarning />
                    {errorMessage}
                  </p>
                ))}
              </div>
              <FloatingArrow
                placement={placement}
                ref={floatingErrorsArrow}
                data={middlewareData.arrow}
              />
            </FloatingBox>
          )}
        </FloatingPortal>
      </>
    );
  },
);

const containerVariants: Variants = {
  idle: ({ hovered, disabled }: ContainerCustom) => {
    const res: Variant = {
      boxShadow: inactiveBoxShadow,
      backgroundColor: ColorsRGB.White,
      opacity: 1,
    };
    if (disabled) {
      res.opacity = 0.5;
      return res;
    }
    if (hovered) {
      res.boxShadow = buttonsBoxShadow;
    }
    return res;
  },
  focused: ({ invalid }: ContainerCustom) => {
    const res = {
      boxShadow: buttonsBoxShadow,
    };
    if (invalid) {
      res.boxShadow = buttonsBoxShadow;
    }
    return res;
  },
};

const inputVariants: Variants = {
  idle: ({ hovered, invalid, disabled }: ContainerCustom) => {
    const res: Variant = {
      borderColor: ColorsRGB.GrayBorder,
    };
    if (disabled) {
      return res;
    }
    if (hovered) {
      res.borderColor = ColorsRGB.GrayLighter;
    }
    if (invalid) {
      res.borderColor = ColorsRGB.Error;
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

const outerLabelVariants: Variants = {
  idle: ({ disabled }: ContainerCustom) => {
    const res: Variant = {
      opacity: 1,
    };
    if (disabled) {
      res.opacity = 0.5;
    }
    return res;
  },
};
