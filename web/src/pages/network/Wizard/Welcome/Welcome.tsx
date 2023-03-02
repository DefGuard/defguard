import './style.scss';

import React from 'react';
import { Link, useNavigate } from 'react-router-dom';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';

const Welcome: React.FC = () => {
  const navigate = useNavigate();
  return (
    <div className="welcome">
      <h1>Welcome to defguard!</h1>
      <p>Before you start, you need to setup your network environment first.</p>
      <Link to={'1'}>
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Setup my network"
        />
      </Link>
      <Button
        onClick={() => navigate('/', { replace: true })}
        size={ButtonSize.BIG}
        styleVariant={ButtonStyleVariant.STANDARD}
        text="Cancel"
      />
    </div>
  );
};

export default Welcome;
