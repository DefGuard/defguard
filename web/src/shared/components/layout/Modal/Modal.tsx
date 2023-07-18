import './style.scss';

import classNames from 'classnames';
import { AnimatePresence, motion } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import ReactDOM from 'react-dom';
import { BehaviorSubject } from 'rxjs';

import { ModalProps } from './types';

type MouseObserverState = {
  press?: React.MouseEvent<HTMLDivElement, MouseEvent>;
  release?: React.MouseEvent<HTMLDivElement, MouseEvent>;
};

/**
 * Modal component, renders it's `children` above all normal page content.
 * This should be used to build generic modal components with proper styling like {@link ModalWithTitle}.
 */
export const Modal = ({
  children,
  setIsOpen,
  className,
  isOpen,
  onClose,
  id,
  currentStep,
  steps,
  afterClose,
  backdrop = true,
  disableClose = false,
}: ModalProps) => {
  const openRef = useRef(isOpen);
  const portalTarget = document.getElementById('modals-root');

  const contentRef = useRef<HTMLDivElement | null>(null);

  const [mouseObserver] = useState(new BehaviorSubject<MouseObserverState>({}));

  useEffect(() => {
    if (mouseObserver && contentRef && isOpen) {
      const sub = mouseObserver.subscribe(({ press, release }) => {
        if (release && press) {
          const checkPress = checkEventOutside(press);
          const checkRelease = checkEventOutside(release);
          if (checkPress && checkRelease && !disableClose) {
            onClose?.();
            setIsOpen?.(false);
          }
        }
      });
      return () => {
        sub.unsubscribe();
      };
    }
  }, [disableClose, isOpen, mouseObserver, onClose, setIsOpen]);

  useEffect(() => {
    // clear observer after closing modal
    if (!isOpen) {
      mouseObserver.next({});
    }
  }, [isOpen, mouseObserver]);

  useEffect(() => {
    openRef.current = isOpen;
  }, [isOpen]);

  /**
   * WORKAROUND
   *
   * Close modal if mouse at click event was outside of modal content box.
   *
   * ClickOutside does not work with react Portals or just in this case, resulting in onClickOutside running when clicked anywhere on the page.
   */
  const checkEventOutside = (
    event: React.MouseEvent<HTMLDivElement, MouseEvent>,
  ): boolean => {
    const domRect = contentRef.current?.getBoundingClientRect();
    if (domRect) {
      const start_x = domRect?.x;
      const start_y = domRect?.y;
      const end_x = start_x + domRect?.width;
      const end_y = start_y + domRect.height;
      if (
        event.clientX < start_x ||
        event.clientX > end_x ||
        event.clientY < start_y ||
        event.clientY > end_y
      ) {
        return true;
      }
    }
    return false;
  };

  const cn = useMemo(() => classNames('modal', className), [className]);

  const stepsEnabled = useMemo(
    () => !isUndefined(steps) && !isUndefined(currentStep),
    [currentStep, steps],
  );

  const [step, setStep] = useState(currentStep);

  const handleAnimationEnd = useCallback(() => {
    if (!openRef.current) {
      afterClose?.();
    }
  }, [afterClose]);

  // This will be used for determining animation direction of modal-content
  useEffect(() => {
    if (steps && !isUndefined(currentStep) && currentStep <= steps?.length) {
      setStep(currentStep);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentStep]);

  if (!portalTarget || !children) return null;

  return ReactDOM.createPortal(
    <AnimatePresence mode="wait">
      {isOpen && (
        <motion.div className="modal-root">
          <motion.div
            className="modal-wrap"
            role="dialog"
            onMouseUp={(event) => {
              if (event) {
                const { press } = mouseObserver.getValue();
                if (press) {
                  mouseObserver.next({ press, release: event });
                }
              }
            }}
            onMouseDown={(event) => {
              if (event) {
                mouseObserver.next({ press: event, release: undefined });
              }
            }}
          >
            <motion.div className={cn}>
              <motion.div
                id={id}
                className="modal-content"
                role="dialog"
                ref={contentRef}
                initial={{
                  opacity: 0,
                }}
                animate={{
                  opacity: 1,
                }}
                exit={{
                  opacity: 0,
                }}
                key={stepsEnabled ? `step-${step}` : 'content'}
                onAnimationComplete={handleAnimationEnd}
              >
                {children}
                {stepsEnabled && steps && !isUndefined(step) ? (
                  <div className={`step-content step-${step}`}>{steps[step]}</div>
                ) : null}
              </motion.div>
            </motion.div>
          </motion.div>
          {backdrop && (
            <motion.div
              className="backdrop"
              initial={{
                opacity: 0,
              }}
              animate={{
                opacity: 0.5,
                y: 0,
              }}
              exit={{
                opacity: 0,
              }}
            ></motion.div>
          )}
        </motion.div>
      )}
    </AnimatePresence>,
    portalTarget,
  );
};
