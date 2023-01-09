import './style.scss';

import { ReactNode } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { DefguardLogo } from '../../svg';
import { Card } from '../Card/Card';

interface Props {
  children?: ReactNode;
}

export const NoLicenseBox = ({ children }: Props) => {
  const { LL } = useI18nContext();
  return (
    <Card className="no-license-box">
      <DefguardLogo />
      {children}
      <br />
      <p>{LL.components.noLicenseBox.footer.get()}</p>
      <p>
        {LL.components.noLicenseBox.footer.contact()}{' '}
        <a href="mailto:sales@defguard.net">sales@defguard.net</a>
      </p>
    </Card>
  );
};
