import './style.scss';

import {
  arrow,
  autoUpdate,
  flip,
  FloatingPortal,
  offset,
  useFloating,
} from '@floating-ui/react';
import classNames from 'classnames';
import { AnimatePresence, HTMLMotionProps, motion } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import React, { ReactNode, useId, useMemo, useRef, useState } from 'react';

import SvgIconWarning from '../../../../../shared/components/svg/IconWarning';
import { FloatingArrow } from '../../../../../shared/defguard-ui/components/Layout/FloatingArrow/FloatingArrow';
import { FloatingBox } from '../../../../../shared/defguard-ui/components/Layout/FloatingBox/FloatingBox';
import { InputFloatingErrors } from '../../../../../shared/defguard-ui/components/Layout/Input/types';

export interface TextareaProps extends HTMLMotionProps<'textarea'> {
  labelExtras?: ReactNode;
  required?: boolean;
  invalid?: boolean;
  label?: string | ReactNode;
  disableOuterLabelColon?: boolean;
  errorMessage?: string;
  floatingErrors?: InputFloatingErrors;
  disposable?: boolean;
  disposeHandler?: (v?: unknown) => void;
}

export const AuthenticationKeyTextField = React.forwardRef<
  HTMLTextAreaElement,
  TextareaProps
>(
  (
    {
      invalid,
      value,
      disposable,
      placeholder,
      disabled = false,
      errorMessage,
      label,
      disableOuterLabelColon,
      floatingErrors,
      labelExtras,
      ...props
    },
    forwardedRef,
  ) => {
    const innerInputRef = useRef<HTMLTextAreaElement | null>(null);

    const inputId = useId();

    const [floatingErrorsOpen, setFloatingErrorsOpen] = useState(false);

    const floatingErrorsArrow = useRef<HTMLDivElement | null>(null);

    const {
      refs: { setFloating },
      placement,
      middlewareData,
      floatingStyles,
    } = useFloating({
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
      return classNames('authentication-key-textarea', {
        invalid,
        disabled,
        disposable: disposable && !disabled,
      });
    }, [disabled, disposable, invalid]);

    return (
      <div className="input">
        {(!isUndefined(label) || !isUndefined(labelExtras)) && (
          <div className="top">
            {!isUndefined(label) && (
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
            {!isUndefined(labelExtras) && labelExtras}
          </div>
        )}
        <motion.div className="authentication-key-textarea-container">
          <motion.textarea
            className={getInputContainerClassName}
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
                ref={setFloating}
                style={floatingStyles}
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
