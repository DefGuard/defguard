import './style.scss';

import clsx from 'clsx';
import { HTMLAttributes, PropsWithChildren } from 'react';

import { Card } from '../../../defguard-ui/components/Layout/Card/Card';

type Props = { title: string } & HTMLAttributes<HTMLDivElement> & PropsWithChildren;

export const SectionWithCard = ({
  title,
  className,
  children,
  ...containerProps
}: Props) => {
  return (
    <div {...containerProps} className={clsx('section-with-title', className)}>
      <h2 className="section-title">{title}</h2>
      <Card>{children}</Card>
    </div>
  );
};
