import './style.scss';

import byteSize from 'byte-size';
import { ComponentPropsWithoutRef, useMemo } from 'react';

export enum NetworkDirection {
  UPLOAD = 'upload',
  DOWNLOAD = 'download',
}

interface Props extends ComponentPropsWithoutRef<'div'> {
  speedValue: number;
  direction?: NetworkDirection;
}

export const NetworkSpeed = ({
  speedValue,
  direction = NetworkDirection.DOWNLOAD,
  className,
  ...rest
}: Props) => {
  const getParsedData = useMemo(() => byteSize(speedValue), [speedValue]);

  const getClassName = useMemo(() => {
    const res = ['network-speed'];
    res.push(direction.valueOf());
    if (className) {
      res.push(className);
    }
    return res.join(' ');
  }, [className, direction]);

  return (
    <div className={getClassName} {...rest}>
      <p className="speed-value">
        {getParsedData.value}
        <span className="measurement">{getParsedData.unit}</span>
      </p>
    </div>
  );
};
