import './style.scss';

import { HTMLMotionProps, motion } from 'framer-motion';
import React from 'react';

const ApplicationVersion: React.FC<HTMLMotionProps<'div'>> = (props) => {
  return (
    <motion.div {...props} className="app-version">
      <p>
        Copyright &copy; 2022{' '}
        <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
          teonite
        </a>
      </p>
    </motion.div>
  );
};

export default ApplicationVersion;
