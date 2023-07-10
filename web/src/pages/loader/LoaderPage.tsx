import './style.scss';

import { LoaderSpinner } from '../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { ColorsRGB } from '../../shared/constants';

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
