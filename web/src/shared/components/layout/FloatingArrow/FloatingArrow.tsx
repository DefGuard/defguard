import './style.scss';

import { Coords, Placement, Side } from '@floating-ui/react-dom-interactions';
import classNames from 'classnames';
import { forwardRef, useMemo } from 'react';

type Props = {
  placement: Placement;
  data?: Partial<Coords> & {
    centerOffset?: number;
  };
};

type Mapping = {
  [key: string]: Side;
};

const mapping: Mapping = {
  top: 'bottom',
  right: 'left',
  bottom: 'top',
  left: 'right',
};

const arrowBase = 8;

export const FloatingArrow = forwardRef<HTMLDivElement, Props>(
  ({ placement, data }, ref) => {
    const currentSide = useMemo(() => {
      const basePlacement = placement.split('-')[0] as string;
      return mapping[basePlacement] as Side;
    }, [placement]);

    const calcX = useMemo(() => {
      switch (currentSide) {
        case 'left':
          return data?.x ?? 0 - arrowBase;
        case 'right':
          return data?.x ?? 0 + arrowBase;
        case 'top':
          return data?.x || arrowBase;
        case 'bottom':
          return data?.x || arrowBase;
      }
    }, [currentSide, data]);

    const calcY = useMemo(() => {
      switch (currentSide) {
        case 'top':
          return data?.y ?? 0 - arrowBase;

        case 'bottom':
          return data?.y ?? 0 + arrowBase;
      }
      return data?.y || arrowBase;
    }, [currentSide, data]);

    const cn = useMemo(() => classNames('floating-ui-arrow', currentSide), [currentSide]);

    return (
      <div
        className={cn}
        ref={ref}
        style={{
          left: calcX,
          top: calcY,
        }}
      ></div>
    );
  },
);
