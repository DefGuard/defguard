import type { PropsWithChildren } from 'react';
import './style.scss';

export const CheckboxGroup = ({ children }: PropsWithChildren) => {
  return <div className="checkbox-group">{children}</div>;
};
