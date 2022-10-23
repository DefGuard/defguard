import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps, motion, SVGMotionProps } from 'framer-motion';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../constants';

interface Props extends HTMLMotionProps<'div'> {
  onDispose?: () => void;
  disposable?: boolean;
  text: string;
}

export const Tag = ({
  onDispose,
  disposable,
  text,
  className,
  ...rest
}: Props) => {
  const cn = useMemo(
    () => classNames('tag', { disposable: disposable }, className),
    [disposable, className]
  );
  const [hoverDismiss, setHoverDismiss] = useState(false);
  return (
    <motion.div className={cn} {...rest}>
      <span>{text}</span>
      {disposable && (
        <motion.button
          className="dispose"
          onClick={() => {
            if (onDispose) {
              onDispose();
            }
          }}
          onHoverStart={() => setHoverDismiss(true)}
          onHoverEnd={() => setHoverDismiss(false)}
        >
          <IconDismiss active={hoverDismiss} />
        </motion.button>
      )}
    </motion.div>
  );
};

interface IconDismissProps extends SVGMotionProps<SVGSVGElement> {
  active?: boolean;
}

const IconDismiss = ({ active = false, ...props }: IconDismissProps) => {
  return (
    <motion.svg
      xmlns="http://www.w3.org/2000/svg"
      width={16}
      height={16}
      role="img"
      {...props}
    >
      <motion.g
        initial="idle"
        animate={active ? 'active' : 'idle'}
        variants={{
          idle: {
            fill: '#cbd3d8',
          },
          active: {
            fill: ColorsRGB.Error,
          },
        }}
      >
        <rect
          data-name="Rectangle 2113"
          width={10}
          height={2}
          rx={1}
          transform="rotate(135 5.05 5.122)"
        />
        <rect
          data-name="Rectangle 2156"
          width={10}
          height={2}
          rx={1}
          transform="rotate(-135 7.95 3.879)"
        />
      </motion.g>
    </motion.svg>
  );
};
