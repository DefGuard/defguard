import { ReactNode } from 'react';

export type CardTabProps = {
  content: ReactNode;
  active?: boolean;
  onClick: () => void;
};
