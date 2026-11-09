import './style.scss';
import clsx from 'clsx';
import { type HTMLAttributes, type PropsWithChildren, useMemo } from 'react';
import { Icon } from '../../defguard-ui/components/Icon';
import type { IconKindValue } from '../../defguard-ui/components/Icon/icon-types';
import { ThemeVariable } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';

type DisplayGridProps = PropsWithChildren & {
  id?: string;
  className?: string;
  testId?: string;
  icon?: IconKindValue;
  label?: string;
};

export const DisplayGrid = ({
  children,
  icon,
  label,
  testId,
  id,
  className,
}: DisplayGridProps) => {
  return (
    <div id={id} className={clsx('display-grid', className)} data-testid={testId}>
      {(isPresent(icon) || isPresent(label)) && (
        <div className="header">
          {isPresent(icon) && (
            <Icon icon={icon} size={20} staticColor={ThemeVariable.FgMuted} />
          )}
          {isPresent(label) && <p className="label">{label}</p>}
        </div>
      )}
      <div className="grid">{children}</div>
    </div>
  );
};

type DisplayGridRowProps = HTMLAttributes<HTMLDivElement>;

export const DisplayGridRow = ({ className, ...props }: DisplayGridRowProps) => {
  return <div className={clsx('row', className)} {...props} />;
};

type DisplayGridCellProps = HTMLAttributes<HTMLDivElement> & {
  span?: number;
};

export const DisplayGridCell = ({
  span,
  className,
  style: outsideStyle,
  ...props
}: DisplayGridCellProps) => {
  const style = useMemo(() => {
    const res: Record<string, number | string> = { ...outsideStyle };
    if (isPresent(span)) res['--span'] = span;
    return res;
  }, [span, outsideStyle]);

  return <div className={clsx('cell', className)} style={style} {...props} />;
};
