import './style.scss';

import {
  arrow,
  autoUpdate,
  flip,
  FloatingPortal,
  offset,
  shift,
  useFloating,
} from '@floating-ui/react';
import classNames from 'classnames';
import { HTMLMotionProps, motion } from 'framer-motion';
import { ReactNode, useMemo, useRef, useState } from 'react';
import ClickAwayListener from 'react-click-away-listener';

import { EditButtonIcon } from './EditButtonIcon';

interface EditButtonProps extends HTMLMotionProps<'button'> {
  disabled?: boolean;
  visible?: boolean;
  children: ReactNode;
}

interface PlacementMap {
  [key: string]: string;
}

/**
 * Replaces OptionsPopover
 * **/
export const EditButton = ({
  children,
  disabled = false,
  visible = true,
  className,
  ...rest
}: EditButtonProps) => {
  const [hovered, setHovered] = useState(false);
  const [open, setOpen] = useState(false);
  const arrowRef = useRef(null);
  const cn = useMemo(
    () =>
      classNames('edit-button', className, {
        visible: visible,
      }),
    [className, visible],
  );
  const { x, y, refs, strategy, placement, middlewareData } = useFloating({
    placement: 'left',
    strategy: 'fixed',
    middleware: [offset(12), flip(), shift(), arrow({ element: arrowRef })],
    open: open,
    onOpenChange: setOpen,
    whileElementsMounted: (refElement, floatingElement, updateFunc) =>
      autoUpdate(refElement, floatingElement, updateFunc),
  });

  const getIconVariant = useMemo(() => {
    if (open && !disabled) {
      return 'hover';
    }
    if (!visible) {
      return 'hidden';
    }
    if (!disabled) {
      if (hovered || open) {
        return 'hover';
      }
    }
    return 'idle';
  }, [disabled, hovered, open, visible]);

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
      <motion.button
        {...rest}
        className={cn}
        onHoverStart={() => setHovered(true)}
        onHoverEnd={() => setHovered(false)}
        onClick={(ev) => {
          ev.preventDefault();
          ev.stopPropagation();
          if (visible) {
            setOpen((state) => !state);
          }
        }}
        disabled={disabled}
        type="button"
      >
        <EditButtonIcon ref={refs.setReference} animate={getIconVariant} />
      </motion.button>
      <FloatingPortal>
        {open && (
          <ClickAwayListener onClickAway={() => setOpen(false)}>
            <motion.div
              className="edit-button-floating-ui"
              ref={refs.setFloating}
              style={{
                position: strategy,
                top: y ?? 0,
                left: x ?? 0,
              }}
              onClick={(env) => {
                env.stopPropagation();
                env.preventDefault();
                setOpen(false);
              }}
            >
              {children}
              <motion.div
                className="arrow"
                data-placement={placement}
                ref={arrowRef}
                style={{
                  left: middlewareData?.arrow?.x ? `${middlewareData.arrow.x}px` : '',
                  top: middlewareData?.arrow?.y ? `${middlewareData.arrow.y}px` : '',
                  right: '',
                  bottom: '',
                  [staticSide]: '-8px',
                }}
              />
            </motion.div>
          </ClickAwayListener>
        )}
      </FloatingPortal>
    </>
  );
};
