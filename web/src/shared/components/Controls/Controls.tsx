import type { PropsWithChildren } from 'react';
import './style.scss';

export const Controls = ({ children }: PropsWithChildren) => {
  return <div className="controls">{children}</div>;
};
