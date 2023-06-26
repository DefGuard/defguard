import './style.scss';

import classNames from 'classnames';
import { motion, TargetAndTransition } from 'framer-motion';
import { ReactNode, useMemo, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { useNavigate } from 'react-router';

import { ColorsRGB } from '../../../shared/constants';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkTabs = () => {
  const navigate = useNavigate();
  const networks = useNetworkPageStore((state) => state.networks);
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);
  const setPageState = useNetworkPageStore((state) => state.setState);

  if (!networks || networks.length === 0) {
    return (
      <div className="network-tabs">
        <Skeleton containerClassName="network-tab-skeleton" />
        <Skeleton containerClassName="network-tab-skeleton" />
        <Skeleton containerClassName="network-tab-skeleton" />
      </div>
    );
  }

  return (
    <div className="network-tabs">
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
      <NetworkTab
        onClick={() => {
          navigate('/admin/wizard', { replace: true });
        }}
        content="+ Add new location"
      />
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
