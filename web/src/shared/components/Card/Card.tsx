import clsx from 'clsx';
import type { HTMLProps, PropsWithChildren } from 'react';

type Props = PropsWithChildren & HTMLProps<HTMLDivElement>;
export const Card = ({ className, children, ...props }: Props) => {
  return (
    <div className={clsx('card', className)} {...props}>
      {children}
    </div>
  );
};
