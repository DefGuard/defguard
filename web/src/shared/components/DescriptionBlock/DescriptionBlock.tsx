import type { HTMLProps, PropsWithChildren } from 'react';
import './style.scss';
import clsx from 'clsx';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';

type Props = HTMLProps<HTMLDivElement> &
  PropsWithChildren & {
    title: string;
  };

export const DescriptionBlock = ({
  children,
  className,
  title,
  ...containerProps
}: Props) => {
  return (
    <div className={clsx('description-block', className)} {...containerProps}>
      <p className="title">{title}</p>
      <SizedBox height={ThemeSpacing.Xs} />
      <div>{children}</div>
    </div>
  );
};
