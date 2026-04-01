import { m } from '../../../../paraglide/messages';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';
import { useApp } from '../../../hooks/useApp';

export const NavTutorialsButton = () => {
  return (
    <button
      type="button"
      className="nav-item"
      onClick={() => useApp.setState({ tutorialsModalOpen: true })}
    >
      <Icon icon="tutorial" />
      <span>{m.cmp_nav_item_tutorials()}</span>
    </button>
  );
};
