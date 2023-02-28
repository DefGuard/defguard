import './style.scss';

import React from 'react';

import StepLocationsForm from './StepLocationsForm';
import StepLocationsList from './StepLocationsList';
const StepLocations: React.FC = () => {
  return (
    <div className="container-basic locations">
      <StepLocationsForm />
      <StepLocationsList />
    </div>
  );
};

export default StepLocations;
