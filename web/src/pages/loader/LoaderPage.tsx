import './style.scss';

import React from 'react';

import LoaderSpinner from '../../shared/components/layout/LoaderSpinner/LoaderSpinner';
import SvgDefguardLogoLogin from '../../shared/components/svg/DefguardLogoLogin';
import { ColorsRGB } from '../../shared/constants';

const LoaderPage: React.FC = () => {
  return (
    <div className="loader-page">
      <div className="logo-container">
        <SvgDefguardLogoLogin />
      </div>
      <LoaderSpinner frontColor={ColorsRGB.White} size={70} />
    </div>
  );
};

export default LoaderPage;
