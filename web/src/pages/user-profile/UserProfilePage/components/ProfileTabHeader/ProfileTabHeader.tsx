import type { PropsWithChildren } from 'react';
import './style.scss';

export const ProfileTabHeader = ({
  children,
  title,
}: PropsWithChildren & {
  title: string;
}) => {
  return (
    <div className="tab-header">
      <h2>{title}</h2>
      <div className="right">{children}</div>
    </div>
  );
};
