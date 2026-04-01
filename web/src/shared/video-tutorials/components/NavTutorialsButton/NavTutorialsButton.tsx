import { m } from '../../../../paraglide/messages';
import { useApp } from '../../../hooks/useApp';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';

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
