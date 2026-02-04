import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import { NavLogo } from '../../Navigation/assets/NavLogo';
import './style.scss';

type Props = {
  onClick?: () => void;
};

export const WizardTop = ({ onClick }: Props) => {
  return (
    <div className="wizard-top">
      <div className="content-track">
        <NavLogo />
        {onClick && <IconButton icon="close" onClick={onClick} />}
      </div>
    </div>
  );
};
