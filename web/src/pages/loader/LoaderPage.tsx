import './style.scss';

import { LoaderSpinner } from '../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';

export const LoaderPage = () => {
  return (
    <div className="loader-page">
      <div className="logo-container">
        <SvgDefguardLogoLogin />
      </div>
      <LoaderSpinner size={70} />
    </div>
  );
};
