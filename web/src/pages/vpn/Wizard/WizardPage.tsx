import './style.scss';

import React from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';
import useBreakpoint from 'use-breakpoint';

import { deviceBreakpoints } from '../../../shared/constants';
import { MobileBanner } from './MobileBanner/MobileBanner';
import Welcome from './Welcome/Welcome';
import WizardLogo from './WizardLogo/WizardLogo';
import Steps from './WizardSteps/WizardSteps';
import PageContainer from '../../../shared/components/layout/PageContainer/PageContainer';

const WizardPage: React.FC = () => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const windowHref = window.location.href.split('/');
  return (
    <PageContainer id="wizard-page">
      <section
        id="wizard"
        className={
          windowHref[windowHref.length - 2] +
            windowHref[windowHref.length - 1] ===
            'wizard' || windowHref[windowHref.length - 1] === 'wizard'
            ? 'center'
            : ''
        }
      >
        <div className="content">
          {breakpoint === 'desktop' && <WizardLogo />}
          {breakpoint !== 'desktop' && <MobileBanner />}

          <div className="steps-container">
            <Routes>
              <Route index element={<Welcome />} />
              <Route path="/:step" element={<Steps />} />
              <Route path="*" element={<Navigate to="" />} />
            </Routes>
          </div>
        </div>
      </section>
    </PageContainer>
  );
};

export default WizardPage;
