import './style.scss';

import classNames from 'classnames';
import { HTMLMotionProps } from 'framer-motion';
import { ReactNode, useMemo } from 'react';

import { Card } from '../Card/Card';
import Divider from '../Divider/Divider';

interface Props extends HTMLMotionProps<'div'> {
  children: ReactNode;
  title?: string;
  footer?: ReactNode;
}

export const ContentCard = ({
  children,
  title,
  footer,
  className,
  ...rest
}: Props) => {
  const cn = useMemo(() => classNames('content-card', className), [className]);
  return (
    <Card className={cn} {...rest}>
      <div className="header">
        <h3> {title} </h3>
      </div>
      <Divider />
      <div className="content">{children}</div>
      {footer ? (
        <>
          <Divider />
          <div className="footer">{footer}</div>
        </>
      ) : null}
    </Card>
  );
};
