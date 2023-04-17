import './style.scss';

import { AnimatePresence, HTMLMotionProps, motion, Variants } from 'framer-motion';
import React, { ReactNode, useState } from 'react';

import { ColorsRGB } from '../../../constants';

export interface Tab {
  title: string;
  node: ReactNode;
}

interface Props {
  tabs: Tab[];
  motionUnderlineID: string;
  headerExtras?: ReactNode;
}

const tabVariants: Variants = {
  enter: (direction: boolean) => {
    return {
      opacity: 0,
      x: direction ? -20 : 20,
    };
  },
  idle: {
    opacity: 1,
    x: 0,
  },
  exit: (direction: boolean) => {
    return {
      opacity: 0,
      x: direction ? -20 : 20,
    };
  },
};

/**
 * Inspired by material design tabs.
 *
 * Switchable content cards by tabs.
 * @param tabs Array of tabs to display.
 * @param motionUnderlineID It is assigned to motion key. This should be UNIQUE in entire application to avoid bugs, but can also be unique in visible content.
 * @param headerExtras Content to insert alongside tab buttons. Only used for mobile right now.
 */
const Tabs: React.FC<HTMLMotionProps<'div'> & Props> = ({
  tabs,
  motionUnderlineID,
  headerExtras,
  ...rest
}) => {
  const [[activeTab, direction], setActiveTab] = useState([0, false]);
  return (
    <motion.div className="tabs-container" {...rest}>
      <header className="controls">
        {tabs.map((tab, index) => (
          <div
            className={index === activeTab ? 'tab active' : 'tab'}
            onClick={() => setActiveTab([index, index > activeTab])}
            key={tab.title}
          >
            <motion.span
              initial={false}
              animate={index === activeTab ? 'active' : 'idle'}
              variants={tabTitleVariants}
              className="tab-title"
            >
              {tab.title}
            </motion.span>
            <span className="default-underline"></span>
            {index === activeTab ? (
              <motion.span
                initial={false}
                layoutId={motionUnderlineID}
                className="active-underline"
              ></motion.span>
            ) : null}
          </div>
        ))}
        {headerExtras}
      </header>
      <AnimatePresence mode="wait">
        <motion.section
          className="content"
          key={activeTab}
          variants={tabVariants}
          animate="idle"
          initial="enter"
          exit="exit"
          transition={{ duration: 0.15 }}
          custom={direction}
        >
          {tabs[activeTab].node}
        </motion.section>
      </AnimatePresence>
    </motion.div>
  );
};

export default Tabs;

const tabTitleVariants: Variants = {
  idle: {
    color: ColorsRGB.GrayDark,
  },
  active: {
    color: ColorsRGB.TextMain,
  },
};
