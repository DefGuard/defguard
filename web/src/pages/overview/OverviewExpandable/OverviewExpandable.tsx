import './style.scss';

import clsx from 'clsx';
import { ReactNode, useState } from 'react';

type Props = {
  children?: ReactNode;
  className?: string;
  id?: string;
  title: string;
};

export const OverviewExpandable = ({ children, title, className, id }: Props) => {
  const [expanded, setExpanded] = useState(true);
  return (
    <div className={clsx('overview-expandable', className)} id={id}>
      <div className="header" onClick={() => setExpanded((s) => !s)}>
        <p>{title}</p>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="22"
          height="22"
          viewBox="0 0 22 22"
          fill="none"
          className={clsx({
            expanded,
          })}
        >
          <path
            d="M5.34276 9.75794L9.5854 14.0006C9.97592 14.3911 10.6091 14.3911 10.9996 14.0006C11.3901 13.6101 11.3901 12.9769 10.9996 12.5864L6.75697 8.34372C6.36645 7.9532 5.73328 7.9532 5.34276 8.34372C4.95223 8.73425 4.95223 9.36741 5.34276 9.75794Z"
            style={{ fill: 'var(--surface-icon-primary)' }}
          />
          <path
            d="M11.3428 13.9994L15.5854 9.75679C15.9759 9.36627 15.9759 8.7331 15.5854 8.34258C15.1949 7.95205 14.5617 7.95205 14.1712 8.34258L9.92855 12.5852C9.53802 12.9757 9.53802 13.6089 9.92855 13.9994C10.3191 14.39 10.9522 14.39 11.3428 13.9994Z"
            style={{ fill: 'var(--surface-icon-primary)' }}
          />
        </svg>
      </div>
      <div
        className={clsx('expandable', {
          expanded,
        })}
      >
        <div>{children}</div>
      </div>
    </div>
  );
};
