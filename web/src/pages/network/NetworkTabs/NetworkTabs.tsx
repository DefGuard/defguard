import './style.scss';

import classNames from 'classnames';
import { motion, TargetAndTransition } from 'framer-motion';
import { ReactNode, useMemo, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { useNavigate } from 'react-router';

import { useI18nContext } from '../../../i18n/i18n-react';
import { ColorsRGB } from '../../../shared/constants';
import { useWizardStore } from '../../wizard/hooks/useWizardStore';
import { useNetworkPageStore } from '../hooks/useNetworkPageStore';

export const NetworkTabs = () => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const networks = useNetworkPageStore((state) => state.networks);
  const selectedNetworkId = useNetworkPageStore((state) => state.selectedNetworkId);
  const setPageState = useNetworkPageStore((state) => state.setState);
  const resetWizardState = useWizardStore((state) => state.resetState);

  if (!networks || networks.length === 0) {
    return (
      <div className="network-tabs">
        <Skeleton containerClassName="network-tab-skeleton" />
        <Skeleton containerClassName="network-tab-skeleton" />
        <Skeleton containerClassName="network-tab-skeleton" />
      </div>
    );
  }

  const handleCreateNetwork = () => {
    resetWizardState();
    navigate('/admin/wizard', { replace: true });
  };

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
      <NetworkTab onClick={handleCreateNetwork} content={LL.networkPage.addNetwork()} />
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
