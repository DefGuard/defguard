import { motion } from 'framer-motion';
import React, { useEffect, useState } from 'react';
import { SVGProps } from 'react';

const SvgIconSuccessLarge = (props: SVGProps<SVGSVGElement>) => {
  const [triggered, setTriggered] = useState(false);

  useEffect(() => {
    setTimeout(() => {
      setTriggered(true);
    }, 500);
  }, []);

  return (
    <svg xmlns="http://www.w3.org/2000/svg" width={108} height={108} {...props}>
      <g
        data-name="Group 4616"
        fill="none"
        stroke="#14bc6e"
        strokeLinecap="round"
        strokeWidth={3}
      >
        <motion.path
          initial={{
            pathLength: 0,
          }}
          animate={{
            pathLength: 1,
            transition: {
              duration: 0.5,
              type: 'tween',
              ease: 'easeInOut',
            },
          }}
          data-name="Line 5396"
          d="m7 54 31.698 32"
        />
        {triggered ? (
          <motion.path
            initial={{
              pathLength: 0,
            }}
            animate={{
              pathLength: 1,
              transition: {
                duration: 0.5,
                type: 'tween',
                ease: 'easeIn',
              },
            }}
            data-name="Line 5397"
            d="m38.698 86 64-64"
          />
        ) : null}
      </g>
    </svg>
  );
};

export default SvgIconSuccessLarge;
