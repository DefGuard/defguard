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
import { AnimatePresence, motion } from 'framer-motion';
import React, { ReactNode, useEffect, useId, useMemo, useRef, useState } from 'react';

import SvgIconAsterix from '../../svg/IconAsterix';
import SvgIconWarning from '../../svg/IconWarning';
import SvgIconX from '../../svg/IconX';
import { FloatingArrow } from '../FloatingArrow/FloatingArrow';
import { FloatingBox } from '../FloatingBox/FloatingBox';
import { InputProps } from './types';

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      required,
      invalid,
      value,
      disposable,
      disposeHandler,
      placeholder,
      disabled = false,
      errorMessage,
      label,
      disableOuterLabelColon,
      floatingErrors,
      ...props
    },
    forwardedRef,
  ) => {
    const innerInputRef = useRef<HTMLInputElement | null>(null);
    const [focused, setFocused] = useState(false);
    const [hovered, setHovered] = useState(false);

    const inputId = useId();

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

    const getInputContainerClassName = useMemo(() => {
      return classNames('input-container', {
        invalid,
        disabled,
        disposable: disposable && !disabled,
      });
    }, [disabled, disposable, invalid]);

    const getInputIcon: ReactNode = useMemo(() => {
      if (disabled) {
        return null;
      }
      if (disposable && (focused || hovered)) {
        return <SvgIconX />;
      }
      if (invalid) {
        return <SvgIconWarning />;
      }
      if (required) {
        return <SvgIconAsterix />;
      }
      return null;
    }, [disabled, disposable, focused, hovered, invalid, required]);

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
      <div className="input">
        {label && (
          <motion.label
            className="input-label"
            htmlFor={inputId}
            initial={false}
            onClick={() => {
              if (innerInputRef) {
                innerInputRef.current?.focus();
              }
            }}
          >
            {label}
            {!disableOuterLabelColon && ':'}
          </motion.label>
        )}
        <motion.div
          ref={reference}
          className={getInputContainerClassName}
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
        >
          <motion.input
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
            id={inputId}
            disabled={disabled}
          />
          <span
            className="icon-container"
            onClick={(e) => {
              if (disposable) {
                e.preventDefault();
                e.stopPropagation();
                disposeHandler?.();
              }
            }}
          >
            {getInputIcon}
          </span>
        </motion.div>
        <AnimatePresence>
          {invalid && errorMessage && !disabled && (
            <motion.span
              className="error-message"
              initial={{
                x: 0,
                opacity: 0,
              }}
              animate={{
                x: 20,
                opacity: 1,
              }}
              exit={{
                opacity: 0,
                x: 0,
              }}
            >
              {errorMessage}
            </motion.span>
          )}
        </AnimatePresence>
        <FloatingPortal>
          <AnimatePresence mode="wait">
            {floatingErrorsOpen && floatingErrors && (
              <FloatingBox
                className="floating-input-errors"
                ref={floating}
                style={{
                  position: strategy,
                  left: x || 0,
                  top: y || 0,
                }}
                initial={{
                  opacity: 0,
                }}
                animate={{
                  opacity: 1,
                }}
                exit={{
                  opacity: 0,
                }}
              >
                <p className="title">{floatingErrors.title}</p>
                <div className="errors">
                  {floatingErrors.errorMessages.map((errorMessage) => (
                    <div className="error" key={errorMessage}>
                      <SvgIconWarning />
                      <p>{errorMessage}</p>
                    </div>
                  ))}
                </div>
                <FloatingArrow
                  placement={placement}
                  ref={floatingErrorsArrow}
                  data={middlewareData.arrow}
                />
              </FloatingBox>
            )}
          </AnimatePresence>
        </FloatingPortal>
      </div>
    );
  },
);
