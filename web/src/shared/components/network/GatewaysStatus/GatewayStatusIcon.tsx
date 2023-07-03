import { motion, TargetAndTransition } from 'framer-motion';
import { useMemo } from 'react';

import { ColorsRGB } from '../../../constants';
import { GatewayConnectionStatus } from './GatewaysStatus';

type Props = {
  status: GatewayConnectionStatus;
  customColor?: ColorsRGB;
};

export const GatewayStatusIcon = ({ status, customColor }: Props) => {
  const getAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      fill: ColorsRGB.Error,
    };
    if (customColor) {
      res.fill = customColor;
      return res;
    }
    switch (status) {
      case GatewayConnectionStatus.CONNECTED:
        res.fill = ColorsRGB.Success;
        break;
      case GatewayConnectionStatus.ERROR:
        res.fill = ColorsRGB.Error;
        break;
      case GatewayConnectionStatus.PARTIAL:
        res.fill = ColorsRGB.Warning;
        break;
      case GatewayConnectionStatus.DISCONNECTED:
        res.fill = ColorsRGB.Error;
        break;
    }
    return res;
  }, [customColor, status]);

  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={8}
      height={8}
      fill="none"
      viewBox="0 0 8 8"
    >
      <motion.path
        animate={getAnimate}
        initial={false}
        fillRule="evenodd"
        d="M4 8a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm0-2a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z"
        clipRule="evenodd"
      />
    </svg>
  );
};
