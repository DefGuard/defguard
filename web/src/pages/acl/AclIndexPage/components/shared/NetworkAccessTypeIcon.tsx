import { NetworkAccessType } from '../../../types';

type Props = {
  type: NetworkAccessType;
};

export const NetworkAccessTypeIcon = ({ type }: Props) => {
  switch (type) {
    case NetworkAccessType.ALLOWED:
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
        >
          <path
            d="M5.575 7.14988L6.75661 9.51309C6.78886 9.57759 6.8767 9.58779 6.92287 9.53239L9.94995 5.89989"
            strokeLinecap="round"
            style={{ stroke: 'var(--surface-positive-primary)' }}
          />
        </svg>
      );
    case NetworkAccessType.DENIED:
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
        >
          <path
            d="M6 6L10 10"
            style={{ stroke: 'var(--surface-alert-primary)' }}
            strokeLinecap="round"
          />
          <path
            d="M10 6L6 10"
            style={{ stroke: 'var(--surface-alert-primary)' }}
            strokeLinecap="round"
          />
        </svg>
      );
    case NetworkAccessType.UNMANAGED:
      return (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
        >
          <circle cx="8" cy="8" r="2" style={{ stroke: 'var(--surface-icon-primary)' }} />
        </svg>
      );
  }
};
