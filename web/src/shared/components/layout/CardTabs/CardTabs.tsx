import './style.scss';

import { ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import Skeleton from 'react-loading-skeleton';

import SvgIconArrowSingle2 from '../../svg/IconArrowSingle2';
import { CardTab } from './components/CardTab';
import { CardTabProps } from './types';

type Props = {
  tabs: (CardTabProps & { key: string | number })[];
  onCreate?: () => void;
  createContent?: ReactNode | string;
  loading?: boolean;
};

export const CardTabs = ({ tabs, onCreate, createContent, loading = false }: Props) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [showScrollControlls, setShowScrollControlls] = useState(false);

  const checkOverflow = useCallback(() => {
    if (containerRef.current) {
      return containerRef.current.scrollWidth > containerRef.current.offsetWidth;
    }
    return false;
  }, []);

  // check overflow on component mount
  useEffect(() => {
    setTimeout(() => {
      if (checkOverflow()) {
        setShowScrollControlls(true);
      } else {
        if (showScrollControlls) {
          setShowScrollControlls(false);
        }
      }
    }, 500);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [containerRef, containerRef.current]);

  if (loading) {
    return (
      <div className="card-tabs">
        <div className="tabs-container">
          <Skeleton containerClassName="network-tab-skeleton" />
          <Skeleton containerClassName="network-tab-skeleton" />
          <Skeleton containerClassName="network-tab-skeleton" />
        </div>
      </div>
    );
  }

  const handleScroll = (direction: 'left' | 'right') => {
    if (containerRef.current) {
      const scrollBy = containerRef.current.offsetWidth * 0.25;
      if (direction === 'left') {
        containerRef.current.scrollBy({
          left: scrollBy * -1,
          behavior: 'smooth',
        });
      } else {
        containerRef.current.scrollBy({
          left: scrollBy,
          behavior: 'smooth',
        });
      }
    }
  };

  return (
    <div className="card-tabs">
      <div className="tabs-container" ref={containerRef}>
        {tabs.map(({ key, ...rest }) => (
          <CardTab {...rest} key={key} />
        ))}
        {onCreate && createContent && (
          <CardTab onClick={() => onCreate()} content={createContent} />
        )}
      </div>
      {showScrollControlls && (
        <div className="scroll-controls">
          <button>
            <SvgIconArrowSingle2 onClick={() => handleScroll('left')} />
          </button>
          <button>
            <SvgIconArrowSingle2 onClick={() => handleScroll('right')} />
          </button>
        </div>
      )}
    </div>
  );
};
