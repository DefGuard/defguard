import './style.scss';

import classNames from 'classnames';
import { motion, Variant, Variants } from 'framer-motion';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../constants';
import { ActionButtonIconCopy } from './icons/ActionButtonIconCopy';
import { ActionButtonIconDownload } from './icons/ActionButtonIconDownload';
import { ActionButtonIconQRCode } from './icons/ActionButtonIconQRCode';

export enum ActionButtonVariant {
  DOWNLOAD = 'download',
  COPY = 'copy',
  QRCODE = 'qrcode',
}

interface Props {
  variant: ActionButtonVariant;
  disabled?: boolean;
  className?: string;
  forcedActive?: boolean;
  onClick?: () => void;
}

/**
 * Styled button holding icon, created for usage with ExpandableCard and RowBox
 * **/
export const ActionButton = ({
  variant,
  className,
  onClick,
  disabled = false,
  forcedActive = false,
}: Props) => {
  const getIcon = useMemo(() => {
    switch (variant) {
      case ActionButtonVariant.COPY:
        return <ActionButtonIconCopy />;
      case ActionButtonVariant.QRCODE:
        return <ActionButtonIconQRCode />;
      case ActionButtonVariant.DOWNLOAD:
        return <ActionButtonIconDownload />;
    }
  }, [variant]);
  const cn = useMemo(
    () =>
      classNames('action-button', className, {
        disabled,
      }),
    [className, disabled]
  );

  const [hovered, setHovered] = useState(false);

  const getAnimate = useMemo(() => {
    if (disabled) {
      return 'idle';
    }
    if (hovered || forcedActive) {
      return 'active';
    }
    return 'idle';
  }, [disabled, forcedActive, hovered]);

  return (
    <motion.button
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
      animate={getAnimate}
      variants={variants}
      className={cn}
      custom={{ disabled }}
      disabled={disabled}
      onClick={() => {
        if (!disabled && onClick) {
          onClick();
        }
      }}
    >
      {getIcon}
    </motion.button>
  );
};

type ButtonCustom = {
  disabled?: boolean;
};

const variants: Variants = {
  idle: ({ disabled }: ButtonCustom) => {
    const res: Variant = {
      backgroundColor: ColorsRGB.BgLight,
      opacity: 1,
    };
    if (disabled) {
      res.opacity = 0.5;
    }
    return res;
  },
  active: ({ disabled }: ButtonCustom) => {
    const res: Variant = {
      opacity: 1,
      backgroundColor: ColorsRGB.Primary,
    };
    if (disabled) {
      res.opacity = 0.5;
    }
    return res;
  },
};
