import './style.scss';

import classNames from 'classnames';
import { motion, TargetAndTransition } from 'framer-motion';
import { ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { useNavigate } from 'react-router';

import { useI18nContext } from '../../../i18n/i18n-react';
import SvgIconArrowSingle2 from '../../../shared/components/svg/IconArrowSingle2';
import { ColorsRGB } from '../../../shared/constants';
import { useWizardStore } from '../../wizard/hooks/useWizardStore';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkTabs = () => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const networks = useNetworkPageStore((state) => state.networks);
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);
  const setPageState = useNetworkPageStore((state) => state.setState);
  const resetWizardState = useWizardStore((state) => state.resetState);
  const [showScrollControlls, setShowScrollControlls] = useState(false);

  const checkOverflow = useCallback(() => {
    if (containerRef.current) {
      return containerRef.current.scrollWidth > containerRef.current.offsetWidth;
    }
    return false;
  }, []);

  // check overflow on component mount
  useEffect(() => {
    setTimeout(() => {
      if (checkOverflow()) {
        setShowScrollControlls(true);
      } else {
        if (showScrollControlls) {
          setShowScrollControlls(false);
        }
      }
    }, 500);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [containerRef, containerRef.current]);

  if (!networks || networks.length === 0) {
    return (
      <div className="network-tabs">
        <div className="tabs-container">
          <Skeleton containerClassName="network-tab-skeleton" />
          <Skeleton containerClassName="network-tab-skeleton" />
          <Skeleton containerClassName="network-tab-skeleton" />
        </div>
      </div>
    );
  }

  const handleCreateNetwork = () => {
    resetWizardState();
    navigate('/admin/wizard', { replace: true });
  };

  const handleScroll = (direction: 'left' | 'right') => {
    if (containerRef.current) {
      const scrollBy = containerRef.current.offsetWidth * 0.25;
      if (direction === 'left') {
        containerRef.current.scrollBy({
          left: scrollBy * -1,
          behavior: 'smooth',
        });
      } else {
        containerRef.current.scrollBy({
          left: scrollBy,
          behavior: 'smooth',
        });
      }
    }
  };

  return (
    <div className="network-tabs">
      <div className="tabs-container" ref={containerRef}>
        {networks.map((n) => (
          <NetworkTab
            onClick={() => {
              if (n.id !== selectedNetworkId) {
                setPageState({ selectedNetworkId: n.id });
              }
            }}
            key={n.id}
            content={n.name}
            active={n.id === selectedNetworkId}
          />
        ))}
        <NetworkTab onClick={handleCreateNetwork} content={LL.networkPage.addNetwork()} />
      </div>
      {showScrollControlls && (
        <div className="scroll-controls">
          <button>
            <SvgIconArrowSingle2 onClick={() => handleScroll('left')} />
          </button>
          <button>
            <SvgIconArrowSingle2 onClick={() => handleScroll('right')} />
          </button>
        </div>
      )}
    </div>
  );
};

type NetworkTabProps = {
  content: ReactNode;
  active?: boolean;
  onClick: () => void;
};

const NetworkTab = ({ onClick, content, active = false }: NetworkTabProps) => {
  const [hovered, setHovered] = useState(false);
  const cn = useMemo(
    () => classNames('network-tab', { active, hovered }),
    [active, hovered]
  );

  const renderContent = useMemo(() => {
    if (typeof content === 'string') {
      return <span>{content}</span>;
    }
    return content;
  }, [content]);

  const getAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      height: 32,
      backgroundColor: ColorsRGB.GrayLighter,
      color: ColorsRGB.GrayLight,
    };

    if (active || hovered) {
      res.height = 42;
      res.color = ColorsRGB.TextMain;
      res.backgroundColor = ColorsRGB.White;
    }

    return res;
  }, [active, hovered]);

  return (
    <motion.button
      initial={false}
      animate={getAnimate}
      className={cn}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onClick}
    >
      {renderContent}
    </motion.button>
  );
};
