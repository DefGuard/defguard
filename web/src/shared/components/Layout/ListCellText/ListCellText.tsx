import './style.scss';

import useResizeObserver from '@react-hook/resize-observer';
import clsx from 'clsx';
import { useCallback, useRef, useState } from 'react';

import { FloatingMenu } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';

type Props = {
  text: string;
};

export const ListCellText = ({ text }: Props) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [overflows, setOverflows] = useState(false);

  const handleResize = useCallback(() => {
    if (containerRef.current) {
      setOverflows(containerRef.current.scrollWidth > containerRef.current.clientWidth);
    }
  }, []);

  useResizeObserver(containerRef, handleResize);
  return (
    <FloatingMenuProvider disabled={!overflows}>
      <div
        className={clsx('list-cell-text', {
          overflows,
        })}
        ref={containerRef}
      >
        <FloatingMenuTrigger asChild>
          <p>{text}</p>
        </FloatingMenuTrigger>
      </div>
      <FloatingMenu className="list-cell-text-floating">
        <p>{text}</p>
      </FloatingMenu>
    </FloatingMenuProvider>
  );
};
