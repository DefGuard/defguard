import { AnimatePresence, motion } from 'framer-motion';
import { Link, useMatch } from 'react-router-dom';

import { NavigationItem } from './Navigation';

interface NavigationLinkProps {
  item: NavigationItem;
  callback?: () => void;
}

export const NavigationLink = ({ item, callback }: NavigationLinkProps) => {
  const match = useMatch(item.linkPath);
  return (
    <Link
      to={item.linkPath}
      onClick={() => callback}
      className={match ? 'active' : undefined}
      replace
    >
      {item.icon}
      <span>{item.title}</span>
      <AnimatePresence>
        {match ? (
          <motion.div
            className="active-line"
            layoutId="navigation-active-line-desktop"
            initial={{
              opacity: 0,
            }}
            animate={{
              opacity: 1,
            }}
            exit={{
              opacity: 0,
            }}
          />
        ) : null}
      </AnimatePresence>
    </Link>
  );
};
