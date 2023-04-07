import './style.scss';

import React from 'react';

import SvgDefguadNavLogo from '../../../shared/components/svg/DefguadNavLogo';

const wizardLogo: React.FC<React.HTMLAttributes<HTMLDivElement>> = (props) => {
  return (
    <div id="wizard-logo" {...props}>
      <SvgDefguadNavLogo />
      <div className="separator" />
      <p>Network setup</p>
    </div>
  );
};

export default wizardLogo;
