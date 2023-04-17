import './style.scss';

import classNames from 'classnames';
import { AnimatePresence, motion } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import React, { ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import ReactDOM from 'react-dom';
import { BehaviorSubject } from 'rxjs';

export interface ModalProps {
  backdrop?: boolean;
  isOpen: boolean;
  setIsOpen: (v: boolean) => void;
  className?: string;
  children?: ReactNode;
  onClose?: () => void;
  id?: string;
  disableClose?: boolean;
  currentStep?: number;
  steps?: ReactNode[];
}

type MouseObserverState = {
  press?: React.MouseEvent<HTMLDivElement, MouseEvent>;
  release?: React.MouseEvent<HTMLDivElement, MouseEvent>;
};

/**
 * Modal component, renders it's `children` above all normal page content.
 * This should be used to build other generic modal components with proper styling like {@link ModalWithTitle}.
 */
const Modal = ({
  children,
  backdrop,
  setIsOpen,
  className,
  isOpen,
  onClose,
  id,
  disableClose = false,
  currentStep,
  steps,
}: ModalProps) => {
  const element = document.getElementById('modals-root');

  const contentRef = useRef<HTMLDivElement | null>(null);

  const [mouseObserver] = useState(new BehaviorSubject<MouseObserverState>({}));

  useEffect(() => {
    if (mouseObserver && contentRef && isOpen) {
      const sub = mouseObserver.subscribe(({ press, release }) => {
        if (release && press) {
          const checkPress = checkEventOutside(press);
          const checkRelease = checkEventOutside(release);
          if (checkPress && checkRelease && !disableClose) {
            setIsOpen(false);
            onClose && onClose();
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

  /**
   * WORKAROUND
   *
   * Close modal if mouse at click event was outside of modal content box.
   *
   * ClickOutside does not work with react Portals or just in this case, resulting in onClickOutside running when clicked anywhere on the page.
   */
  const checkEventOutside = (
    event: React.MouseEvent<HTMLDivElement, MouseEvent>
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
    [currentStep, steps]
  );

  const [step, setStep] = useState(currentStep);

  // This will be used for determining animation direction of modal-content
  useEffect(() => {
    if (steps && !isUndefined(currentStep) && currentStep <= steps?.length) {
      setStep(currentStep);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentStep]);

  if (!element || !children) return null;
  return ReactDOM.createPortal(
    <AnimatePresence>
      {isOpen ? (
        <motion.div className="modal-root">
          <motion.div
            className="modal-wrap"
            role="dialog"
            tabIndex={-1}
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
              >
                {children}
                {stepsEnabled && steps && !isUndefined(step) ? (
                  <div className={`step-content step-${step}`}>{steps[step]}</div>
                ) : null}
              </motion.div>
            </motion.div>
          </motion.div>
          {backdrop ? (
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
          ) : null}
        </motion.div>
      ) : null}
    </AnimatePresence>,
    element
  );
};

export default Modal;
