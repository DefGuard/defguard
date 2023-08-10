import './style.scss';

import { saveAs } from 'file-saver';
import { AnimatePresence, motion, Variants } from 'framer-motion';
import { useEffect, useState } from 'react';
import { Subject, switchMap, timer } from 'rxjs';

import SvgIconCopy from '../../../../../shared/components/svg/IconCopy';
import SvgIconDownload from '../../../../../shared/components/svg/IconDownload';
import SvgIconUserListExpanded from '../../../../../shared/components/svg/IconUserListExpanded';
import SvgIconUserListHover from '../../../../../shared/components/svg/IconUserListHover';
import { ColorsRGB } from '../../../../../shared/constants';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useClipboard } from '../../../../../shared/hooks/useClipboard';

interface Props {
  title: string;
  keyValue?: string;
  collapsible?: boolean;
  disabled?: boolean;
  initiallyOpen?: boolean;
}

const collapsedVariants: Variants = {
  collapsed: {
    height: 0,
  },
  active: {
    height: 'auto',
  },
};

const titleVariants: Variants = {
  collapsed: {
    color: ColorsRGB.TextMain,
  },
  active: {
    color: ColorsRGB.GrayDark,
  },
};

const KeyBox = ({
  title,
  keyValue,
  collapsible = false,
  disabled = false,
  initiallyOpen,
}: Props) => {
  const [collapsed, setCollapsed] = useState(initiallyOpen);
  const [copiedVisible, setCopiedVisible] = useState(false);
  const [copySubject, setCopySubject] = useState<Subject<void> | undefined>();
  const { writeToClipboard } = useClipboard();

  const handleCopy = () => {
    if (keyValue) {
      writeToClipboard(keyValue).then(() => {
        setCopiedVisible(true);
        copySubject?.next();
      });
    }
  };

  const handleDownload = () => {
    const blob = new Blob([keyValue as string], {
      type: 'text/plain;charset=utf-8',
    });
    saveAs(blob, `${title.replace(' ', '_').toLocaleLowerCase()}.txt`);
  };

  const handleClick = () => {
    if (!disabled) {
      setCollapsed((state) => !state);
    }
  };

  useEffect(() => {
    if (!copySubject) {
      setCopySubject(new Subject());
    } else {
      const sub = copySubject.pipe(switchMap(() => timer(2500))).subscribe(() => {
        setCopiedVisible(false);
      });
      return () => {
        sub.unsubscribe();
      };
    }
  }, [copySubject]);

  if (!keyValue) return null;

  return (
    <div>
      <motion.div className="key-box">
        <div className="top">
          {collapsible ? (
            <div
              className={`collapse-controller ${disabled ? 'disabled' : ''}`}
              onClick={handleClick}
            >
              {collapsed ? <SvgIconUserListExpanded /> : <SvgIconUserListHover />}
            </div>
          ) : null}
          <motion.span
            className="title"
            variants={titleVariants}
            animate={collapsed ? 'collapsed' : 'active'}
            onClick={handleClick}
          >
            {title}
          </motion.span>
          <motion.div className="actions">
            <Button
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.ICON}
              onClick={handleCopy}
              disabled={disabled}
            >
              <SvgIconCopy />
            </Button>
            <Button
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.ICON}
              onClick={handleDownload}
              disabled={disabled}
            >
              <SvgIconDownload />
            </Button>
          </motion.div>
        </div>
        {collapsible && collapsed ? (
          <motion.div
            variants={collapsedVariants}
            animate="active"
            className="key-container"
            layout
          >
            <p className="key">{keyValue}</p>
          </motion.div>
        ) : null}
      </motion.div>
      <AnimatePresence>
        {copiedVisible ? (
          <motion.span
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="clipboard-notification"
          >
            Copied to clipboard!
          </motion.span>
        ) : (
          <div style={{ height: 23 }} />
        )}
      </AnimatePresence>
    </div>
  );
};

export default KeyBox;
