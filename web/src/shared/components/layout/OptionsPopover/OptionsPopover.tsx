import './style.scss';

import { AnimatePresence, motion } from 'framer-motion';
import React, { ReactNode, useEffect, useState } from 'react';
import ClickAwayListener from 'react-click-away-listener';
import { usePopper } from 'react-popper';

import { standardVariants } from '../../../variants';

export type popperOptions = Omit<typeof usePopper, 'options'>;

interface Props {
  items: ReactNode[];
  referenceElement: Element;
  isOpen: boolean;
  setIsOpen: (v: boolean) => void;
  popperOptions?: popperOptions;
}

/**
 * DEPRACATED! by EditButton
 * Based on Popper.js, displays menu with options.
 * @param items Items passed to display inside, intended to use with `<button>` elements but can be anything that can be rendered as a child of `<li>`.
 * @param referenceElement Menu will position itself based on this element. Should be set using `useState` according to popper docs.
 * @param isOpen Determinate visibility of menu.
 * @param popperOptions Overrides default options such as positioning. See popperOptions for more.
 */
const OptionsPopover: React.FC<Props> = ({
  referenceElement,
  items,
  popperOptions,
  isOpen,
  setIsOpen,
}) => {
  const [popperElement, setPopperElement] = useState<HTMLDivElement | null>(null);
  const [arrowElement, setArrowElement] = useState<HTMLDivElement | null>(null);
  const { styles, attributes } = usePopper(referenceElement, popperElement, {
    strategy: 'fixed',
    placement: 'right',
    ...popperOptions,
    modifiers: [
      { name: 'arrow', options: { element: arrowElement } },
      {
        name: 'offset',
        options: {
          offset: [0, 10],
        },
      },
    ],
  });

  useEffect(() => {
    if (isOpen) {
      referenceElement.className = `${referenceElement.className} active`;
    } else {
      referenceElement.className = referenceElement.className.replace('active', '');
    }
  }, [isOpen, referenceElement]);

  useEffect(() => {
    const handleClick = () => {
      setIsOpen(true);
    };
    referenceElement.addEventListener('click', handleClick);
    return () => referenceElement.removeEventListener('click', handleClick);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return (
    <AnimatePresence>
      {isOpen ? (
        <ClickAwayListener onClickAway={() => setIsOpen(false)}>
          <motion.div
            initial="hidden"
            animate="show"
            exit="hidden"
            variants={standardVariants}
            className="popover-basic"
            ref={setPopperElement}
            style={styles.popper}
            {...attributes.popper}
          >
            <ul>
              {items.map((item, index) => (
                <li key={index}>{item}</li>
              ))}
            </ul>
            <div
              ref={setArrowElement}
              className="arrow"
              style={styles.arrow}
              {...attributes.arrow}
            ></div>
          </motion.div>
        </ClickAwayListener>
      ) : null}
    </AnimatePresence>
  );
};

export default OptionsPopover;
