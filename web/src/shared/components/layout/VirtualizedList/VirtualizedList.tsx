import './style.scss';

import { useVirtualizer } from '@tanstack/react-virtual';
import classNames from 'classnames';
import { motion, Variants } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { ReactNode, useMemo, useRef, useState } from 'react';
import useBreakpoint from 'use-breakpoint';

import { ColorsRGB, deviceBreakpoints } from '../../../constants';
import { VirtualizedListSortIcon } from './VirtualizedListSortIcon';

export const VirtualizedList = <T extends object>({
  className,
  id,
  headers,
  rowSize,
  data,
  cells,
  customRowRender,
  mobile,
  padding,
  headerPadding,
}: Props<T>) => {
  const listRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: data.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => rowSize,
  });

  const cn = useMemo(
    () => classNames('virtualized-list-container', className),
    [className]
  );

  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const renderRow = (value: T) => {
    if (breakpoint !== 'desktop' && mobile?.enabled && mobile.renderer) {
      return mobile.renderer(value);
    }
    if (customRowRender) {
      return customRowRender(value);
    }
    return (
      <DefaultRowRender>
        {cells?.map(({ render, onClick }, index) => (
          <div
            className={`cell-${index}`}
            key={index}
            onClick={() => {
              if (onClick) {
                onClick(value);
              }
            }}
          >
            {render(value)}
          </div>
        ))}
      </DefaultRowRender>
    );
  };
  const rowWidth = useMemo(() => {
    if (padding && (padding.left || padding.right)) {
      let res = 0;
      if (padding.left) {
        res += padding.left;
      }
      if (padding.right) {
        res += padding.right;
      }
      return `calc(100% - ${res}px)`;
    }
    return '100%';
  }, [padding]);

  return (
    <div className={cn} id={id}>
      {headers && headers.length > 0 && (
        <div
          className="headers"
          style={{
            paddingBottom: headerPadding?.bottom || 0,
            paddingTop: headerPadding?.top || 0,
            paddingLeft: (padding?.left || 0) + (headerPadding?.left || 0),
            paddingRight: (padding?.right || 0) + (headerPadding?.right || 0),
          }}
        >
          {headers.map((header) => (
            <ListHeader {...header} key={header.key} />
          ))}
        </div>
      )}
      <div
        className="scroll-container"
        ref={listRef}
        style={{
          overflow: 'auto',
        }}
      >
        <div
          className="rows-container"
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: '100%',
            position: 'relative',
            paddingBottom: padding?.bottom || 0,
            paddingTop: padding?.top || 0,
            paddingLeft: padding?.left || 0,
            paddingRight: padding?.right || 0,
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualItem) => (
            <div
              className="virtual-row"
              key={virtualItem.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: rowWidth,
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px) translateX(${
                  padding?.left || 0
                }px)`,
              }}
            >
              {renderRow(data[virtualItem.index])}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

const ListHeader = ({ text, onClick, sortDirection, active }: ListHeader) => {
  const getIconAnimate = useMemo(() => {
    if (active) {
      switch (sortDirection) {
        case ListSortDirection.ASC:
          return 'asc';
        case ListSortDirection.DESC:
          return 'desc';
        default:
          return 'desc';
      }
    }
    return 'idle';
  }, [active, sortDirection]);

  const clickable = useMemo(() => !isUndefined(onClick), [onClick]);

  const cn = useMemo(
    () =>
      classNames('header', {
        active,
        clickable,
      }),
    [active, clickable]
  );

  return (
    <div
      className={cn}
      onClick={() => {
        if (onClick && active) {
          onClick();
        }
      }}
    >
      <motion.span
        variants={headerSpanVariants}
        animate={active ? 'active' : 'idle'}
      >
        {text}
      </motion.span>
      <VirtualizedListSortIcon
        className={getIconAnimate}
        animate={getIconAnimate}
        variants={headerSortIconVariants}
        initial={false}
      />
    </div>
  );
};

type DefaultRowRenderProps = {
  children?: ReactNode;
};

const DefaultRowRender = ({ children }: DefaultRowRenderProps) => {
  const [hovered, setHovered] = useState(false);
  return (
    <motion.div
      variants={defaultRowContainerVariants}
      className="default-row"
      initial={false}
      animate={hovered ? 'hovered' : 'idle'}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      {children}
    </motion.div>
  );
};

const defaultRowContainerVariants: Variants = {
  idle: {
    boxShadow: '5px 10px 20px rgba(0,0,0,0)',
  },
  hovered: {
    boxShadow: '5px 10px 20px rgba(0,0,0,0.1)',
  },
};

const headerSpanVariants: Variants = {
  idle: {
    color: ColorsRGB.GrayLight,
  },
  active: {
    color: ColorsRGB.TextMain,
  },
};

const headerSortIconVariants: Variants = {
  idle: {
    opacity: 0,
    rotateZ: 0,
  },
  asc: {
    opacity: 1,
    rotateZ: 180,
  },
  desc: {
    opacity: 1,
    rotateZ: 0,
  },
};

export enum ListSortDirection {
  ASC = 'ASC',
  DESC = 'DESC',
}

export type ListHeader = {
  text: string;
  key: string;
  active?: boolean;
  sortDirection?: ListSortDirection;
  onClick?: () => void;
};

export type ListRowCell<T extends object> = {
  key: string;
  render: (context: T) => ReactNode;
  onClick?: (context: T) => void;
};

export type ListPadding = {
  top?: number;
  bottom?: number;
  left?: number;
  right?: number;
};

interface Props<T extends object> {
  rowSize: number;
  data: T[];
  headers?: ListHeader[];
  cells?: ListRowCell<T>[];
  customRowRender?: (context: T) => ReactNode;
  className?: string;
  id?: string;
  mobile?: {
    enabled: boolean;
    mobileRowSize: number;
    renderer: (context: T) => ReactNode;
  };
  padding?: ListPadding;
  headerPadding?: ListPadding;
}
