import './style.scss';

import {
  arrow,
  autoUpdate,
  flip,
  offset,
  shift,
  useFloating,
} from '@floating-ui/react-dom-interactions';
import { AnimatePresence, motion } from 'framer-motion';
import { ReactNode, useMemo, useRef, useState } from 'react';
import ClickAwayListener from 'react-click-away-listener';

import { IconInfo } from '../../svg';

interface PlacementMap {
  [key: string]: string;
}

interface Props {
  children?: ReactNode;
  initialPlacement?: 'left' | 'right';
}

export const Helper = ({ children, initialPlacement = 'right' }: Props) => {
  const [floatOpen, setFloatOpen] = useState(false);
  const arrowRef = useRef(null);
  const { x, y, strategy, floating, reference, placement, middlewareData } =
    useFloating({
      placement: initialPlacement,
      strategy: 'fixed',
      open: floatOpen,
      onOpenChange: setFloatOpen,
      whileElementsMounted: (refElement, floatingElement, updateFunc) =>
        autoUpdate(refElement, floatingElement, updateFunc),
      middleware: [flip(), offset(16), shift(), arrow({ element: arrowRef })],
    });

  const staticSide: string = useMemo(() => {
    const mapping: PlacementMap = {
      top: 'bottom',
      right: 'left',
      bottom: 'top',
      left: 'right',
    };
    const basePlacement = placement.split('-')[0] as string;
    return mapping[basePlacement] as string;
  }, [placement]);

  return (
    <>
      <button
        className="helper"
        onClick={() => setFloatOpen(true)}
        ref={reference}
      >
        <IconInfo />
      </button>
      <AnimatePresence mode="wait">
        {floatOpen && (
          <ClickAwayListener onClickAway={() => setFloatOpen(false)}>
            <motion.div
              className="helper-floating"
              ref={floating}
              style={{
                position: strategy,
                top: y ?? 0,
                left: x ?? 0,
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
              transition={{
                duration: 0.2,
              }}
            >
              {children}
              <div
                className="arrow"
                data-placement={placement}
                ref={arrowRef}
                style={{
                  left: middlewareData?.arrow?.x
                    ? `${middlewareData.arrow.x}px`
                    : '',
                  top: middlewareData?.arrow?.y
                    ? `${middlewareData.arrow.y}px`
                    : '',
                  right: '',
                  bottom: '',
                  [staticSide]: '-8px',
                }}
              />
            </motion.div>
          </ClickAwayListener>
        )}
      </AnimatePresence>
    </>
  );
};
