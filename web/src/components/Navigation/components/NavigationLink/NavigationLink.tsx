import { useMatch } from 'react-router';
import { Link } from 'react-router-dom';

import { NavigationItem } from '../../types';

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
      {match ? <div className="active-line" /> : null}
    </Link>
  );
};
