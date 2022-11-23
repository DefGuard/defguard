import './style.scss';

import { HTMLMotionProps, motion } from 'framer-motion';
import React from 'react';

import { useAppStore } from '../../../shared/hooks/store/useAppStore';

const ApplicationVersion: React.FC<HTMLMotionProps<'div'>> = (props) => {
  const version = useAppStore((store) => store.version);
  return (
    <motion.div {...props} className="app-version">
      <p>
        Copyright &copy; 2022{' '}
        <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
          teonite
        </a>
      </p>
      {version && <p>Application version: {version}</p>}
    </motion.div>
  );
};

export default ApplicationVersion;
