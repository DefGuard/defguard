import { type PropsWithChildren, useMemo } from 'react';
import './style.scss';
import clsx from 'clsx';
import { IconDesktop } from './icons/IconDesktop';
import { IconFile } from './icons/IconFile';
import { IconGlobe } from './icons/IconGlobe';
import { IconPhone } from './icons/IconPhone';

export const ContainerWithIcon = ({
  children,
  className,
  id,
  icon,
}: PropsWithChildren & {
  icon: 'phone' | 'file' | 'globe' | 'desktop';
  className?: string;
  id?: string;
}) => {
  const RenderIcon = useMemo(() => {
    switch (icon) {
      case 'desktop':
        return IconDesktop;
      case 'file':
        return IconFile;
      case 'globe':
        return IconGlobe;
      case 'phone':
        return IconPhone;
    }
  }, [icon]);

  return (
    <div id={id} className={clsx('container-with-icon', className)}>
      <div className="track">
        <div className="container-icon" data-icon={icon}>
          <RenderIcon />
        </div>
        <div className="content">{children}</div>
      </div>
    </div>
  );
};
