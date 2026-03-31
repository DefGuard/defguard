import { m } from '../../../../paraglide/messages';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';

export const NavTutorialsButton = () => {
  return (
    <button type="button" className="nav-item">
      <Icon icon="tutorial" />
      <span>{m.cmp_nav_item_tutorials()}</span>
    </button>
  );
};
