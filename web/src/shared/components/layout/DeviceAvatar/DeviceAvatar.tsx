import './style.scss';

import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import { useMemo } from 'react';

import { ColorsRGB } from '../../../constants';
import SvgAvatar01Blue from '../../svg/Avatar01Blue';
import SvgAvatar01Gray from '../../svg/Avatar01Gray';
import SvgAvatar02Blue from '../../svg/Avatar02Blue';
import SvgAvatar02Gray from '../../svg/Avatar02Gray';
import SvgAvatar03Blue from '../../svg/Avatar03Blue';
import SvgAvatar03Gray from '../../svg/Avatar03Gray';
import SvgAvatar04Blue from '../../svg/Avatar04Blue';
import SvgAvatar04Gray from '../../svg/Avatar04Gray';
import SvgAvatar05Blue from '../../svg/Avatar05Blue';
import SvgAvatar05Gray from '../../svg/Avatar05Gray';
import SvgAvatar06Blue from '../../svg/Avatar06Blue';
import SvgAvatar06Gray from '../../svg/Avatar06Gray';
import SvgAvatar07Blue from '../../svg/Avatar07Blue';
import SvgAvatar07Gray from '../../svg/Avatar07Gray';
import SvgAvatar08Blue from '../../svg/Avatar08Blue';
import SvgAvatar08Gray from '../../svg/Avatar08Gray';
import SvgAvatar09Blue from '../../svg/Avatar09Blue';
import SvgAvatar09Gray from '../../svg/Avatar09Gray';
import SvgAvatar10Blue from '../../svg/Avatar10Blue';
import SvgAvatar10Gray from '../../svg/Avatar10Gray';
import SvgAvatar11Blue from '../../svg/Avatar11Blue';
import SvgAvatar11Gray from '../../svg/Avatar11Gray';
import SvgAvatar12Blue from '../../svg/Avatar12Blue';
import SvgAvatar12Gray from '../../svg/Avatar12Gray';
import { getDeviceAvatar } from './utils/getDeviceAvatar';

export enum DeviceAvatarVariants {
  BLANK = 'blank',
  GRAY_BOX = 'grayBox',
}

interface Props extends HTMLMotionProps<'div'> {
  active?: boolean;
  styleVariant?: DeviceAvatarVariants;
  deviceId?: number;
}

// NOTE: This matter should be discussed later.
// Each avatar contains 12 svg parts that when displayed together makes whole shape.
// Each device should have some identifier that will determinate the shape of it's avatar.

const blue: JSX.Element[] = [
  <SvgAvatar01Blue key={1} />,
  <SvgAvatar02Blue key={2} />,
  <SvgAvatar03Blue key={3} />,
  <SvgAvatar04Blue key={4} />,
  <SvgAvatar05Blue key={5} />,
  <SvgAvatar06Blue key={6} />,
  <SvgAvatar07Blue key={7} />,
  <SvgAvatar08Blue key={8} />,
  <SvgAvatar09Blue key={9} />,
  <SvgAvatar10Blue key={10} />,
  <SvgAvatar11Blue key={11} />,
  <SvgAvatar12Blue key={12} />,
];
const gray: JSX.Element[] = [
  <SvgAvatar01Gray key={1} />,
  <SvgAvatar02Gray key={2} />,
  <SvgAvatar03Gray key={3} />,
  <SvgAvatar04Gray key={4} />,
  <SvgAvatar05Gray key={5} />,
  <SvgAvatar06Gray key={6} />,
  <SvgAvatar07Gray key={7} />,
  <SvgAvatar08Gray key={8} />,
  <SvgAvatar09Gray key={9} />,
  <SvgAvatar10Gray key={10} />,
  <SvgAvatar11Gray key={11} />,
  <SvgAvatar12Gray key={12} />,
];

/**
 * Displays avatar for user devices.
 * @param active Determinate style variant.
 */
export const DeviceAvatar = ({
  active = true,
  styleVariant = DeviceAvatarVariants.BLANK,
  deviceId,
  ...props
}: Props) => {

  const deviceAvatar = useMemo(() => {
    if (deviceId) {
      const elements = getDeviceAvatar(deviceId);
      const result: JSX.Element[] = blue.filter((el) => {
        if (!elements.includes(Number(el.key))) {
          return true;
        }
      });
      return result as JSX.Element[];
    }
  }, [deviceId]);

  const avatar = useMemo(() => {
    if (active) {
      if (deviceId && deviceAvatar) {
        return deviceAvatar;
      } else {
        return blue;
      }
    } else {
      return gray;
    }
  }, [active, deviceAvatar, deviceId]);

  const getClassName = useMemo(() => {
    const res = ['avatar-icon'];
    if (active) {
      res.push('active');
    }
    res.push(styleVariant.valueOf());
    return res.join(' ');
  }, [active, styleVariant]);

  const getAnimate = useMemo(() => {
    return styleVariant.valueOf();
  }, [styleVariant]);

  return (
    <motion.div
      {...props}
      variants={containerVariants}
      className={getClassName}
      initial={false}
      animate={getAnimate}
    >
      {avatar}
    </motion.div>
  );
};

const containerVariants: Variants = {
  blank: {
    backgroundColor: 'transparent',
    borderRadius: '0px',
  },
  grayBox: {
    backgroundColor: ColorsRGB.BgLight,
    borderRadius: '10px',
  },
};
