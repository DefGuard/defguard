import './style.scss';
import clsx from 'clsx';
import type { HTMLProps, PropsWithChildren } from 'react';

interface Props extends PropsWithChildren {
  containerProps?: HTMLProps<HTMLDivElement>;
}

export const PlaygroundPolicyInfoListTest = ({ containerProps, children }: Props) => {
  return (
    <div
      {...containerProps}
      className={clsx('playground-policy-info-list-test', containerProps?.className)}
    >
      <div className="grid-track">{children}</div>
    </div>
  );
};
