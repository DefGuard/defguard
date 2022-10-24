import './style.scss';

import { HTMLMotionProps, motion } from 'framer-motion';
import React from 'react';

import { useAppStore } from '../../../shared/hooks/store/useAppStore';

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
const appVersion = import.meta.env.PACKAGE_VERSION;

const ApplicationVersion: React.FC<HTMLMotionProps<'div'>> = (props) => {
  const backendVersion = useAppStore((state) => state.backendVersion);

  return (
    <motion.div {...props} className="AppVersion">
      <motion.p>
        Copyright @ 2022 TEONITE
        <span>
          {backendVersion ? `Backend v${backendVersion}` : null}
          {appVersion ? ` :: Frontend ${appVersion}` : null}
        </span>
      </motion.p>
    </motion.div>
  );
};

export default ApplicationVersion;
