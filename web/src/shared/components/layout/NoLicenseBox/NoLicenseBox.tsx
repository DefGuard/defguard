import './style.scss';

import { ReactNode } from 'react';

import { DefguardLogo } from '../../svg';
import { Card } from '../Card/Card';

interface Props {
  children?: ReactNode;
}

export const NoLicenseBox = ({ children }: Props) => {
  return (
    <Card className="no-license-box">
      <DefguardLogo />
      {children}
      <br />
      <p>Get an enterprise license</p>
      <p>
        by contacting:{' '}
        <a href="mailto:sales@defguard.net">sales@defguard.net</a>
      </p>
    </Card>
  );
};
