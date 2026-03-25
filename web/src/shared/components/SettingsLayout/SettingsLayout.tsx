import type { HTMLProps, PropsWithChildren, ReactNode } from 'react';
import './style.scss';
import clsx from 'clsx';
import { LayoutGrid } from '../LayoutGrid/LayoutGrid';

type Props = HTMLProps<HTMLDivElement> &
  PropsWithChildren & {
    suggestion?: ReactNode;
  };

export const SettingsLayout = ({ children, className, suggestion, ...props }: Props) => {
  return (
    <div
      className={clsx('settings-layout', {
        'with-suggestion': suggestion,
      })}
    >
      <LayoutGrid variant="default">
        <div className={clsx('main-content', className)} {...props}>
          {children}
        </div>
        {suggestion && <div className="suggestion-content">{suggestion}</div>}
      </LayoutGrid>
    </div>
  );
};
