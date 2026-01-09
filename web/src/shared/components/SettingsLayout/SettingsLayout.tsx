import type { HTMLProps, PropsWithChildren } from 'react';
import './style.scss';
import clsx from 'clsx';
import { LayoutGrid } from '../LayoutGrid/LayoutGrid';

export const SettingsLayout = ({
  children,
  className,
  ...props
}: PropsWithChildren & HTMLProps<HTMLDivElement>) => {
  return (
    <div className="settings-layout">
      <LayoutGrid variant="default">
        <div className={clsx('main-content', className)} {...props}>
          {children}
        </div>
      </LayoutGrid>
    </div>
  );
};
