import './style.scss';
import type { PropsWithChildren } from 'react';

type Props = {
  cols?: number;
} & PropsWithChildren;
export const FormRow = ({ children, cols = 2 }: Props) => {
  return (
    <div
      className="form-row"
      style={{
        gridTemplateColumns: `repeat(${cols}, 1fr)`,
      }}
    >
      {children}
    </div>
  );
};
