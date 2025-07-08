import './style.scss';

import { Placement } from '@floating-ui/react';
import useResizeObserver from '@react-hook/resize-observer';
import clsx from 'clsx';
import { useCallback, useRef, useState } from 'react';

import { ActionButton } from '../../../defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../defguard-ui/components/Layout/ActionButton/types';
import { FloatingMenu } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import { useClipboard } from '../../../hooks/useClipboard';

type Props = {
  text: string;
  withCopy?: boolean;
  placement?: Placement;
};

export const ListCellText = ({ text, withCopy, placement = 'left' }: Props) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [overflows, setOverflows] = useState(false);

  const { writeToClipboard } = useClipboard();

  const handleResize = useCallback(() => {
    if (containerRef.current) {
      setOverflows(containerRef.current.scrollWidth > containerRef.current.clientWidth);
    }
  }, []);

  useResizeObserver(containerRef, handleResize);
  return (
    <FloatingMenuProvider disabled={!overflows} placement={placement}>
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
      <FloatingMenu
        className={clsx('list-cell-text-floating', {
          copy: withCopy,
        })}
      >
        <p>{text}</p>
        {withCopy && (
          <ActionButton
            variant={ActionButtonVariant.COPY}
            onClick={() => {
              void writeToClipboard(text);
            }}
          />
        )}
      </FloatingMenu>
    </FloatingMenuProvider>
  );
};
