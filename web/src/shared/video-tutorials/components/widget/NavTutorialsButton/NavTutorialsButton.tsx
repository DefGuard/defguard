import { m } from '../../../../../paraglide/messages';
import { Icon } from '../../../../defguard-ui/components/Icon/Icon';
import { useVideoTutorialsModal } from '../../../store';

export const NavTutorialsButton = () => {
  return (
    <button
      type="button"
      className="nav-item"
      onClick={() => useVideoTutorialsModal.setState({ isOpen: true })}
    >
      <Icon icon="tutorial" />
      <span>{m.cmp_nav_item_tutorials()}</span>
    </button>
  );
};
