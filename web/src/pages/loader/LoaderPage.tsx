import './style.scss';

import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { ColorsRGB } from '../../shared/constants';
import { LoaderSpinner } from '../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';

export const LoaderPage = () => {
  return (
    <div className="loader-page">
      <div className="logo-container">
        <SvgDefguardLogoLogin />
      </div>
      <LoaderSpinner color={ColorsRGB.White} size={70} />
    </div>
  );
};
