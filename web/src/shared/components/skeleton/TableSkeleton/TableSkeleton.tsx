import './style.scss';
import { useWindowSize } from '@uidotdev/usehooks';
import { useMemo, useRef } from 'react';
import Skeleton from 'react-loading-skeleton';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';

export const TableSkeleton = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const windowHeight = useWindowSize().height;

  const initHeight = useMemo(() => {
    if (!containerRef.current) return 0;
    return window.innerHeight - containerRef.current.getBoundingClientRect().top;
  }, []);

  const minHeight = useMemo(() => {
    const container = containerRef.current;
    if (!container || !windowHeight) return null;
    return windowHeight - container.getBoundingClientRect().top - 20;
  }, [windowHeight]);

  return (
    <div className="table-skeleton-wrapper">
      <Skeleton height={40} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <div
        className="table-skeleton"
        ref={containerRef}
        style={{
          minHeight: minHeight ?? initHeight,
        }}
      >
        <Skeleton height={minHeight ?? 0} />
      </div>
    </div>
  );
};
