import businessImage from './assets/business.png';
import enterpriseImage from './assets/enterprise.png';
import expiredImage from './assets/expired.png';
import limitImage from './assets/limit.png';
import type { LicenseModalSideImageVariantValue } from './types';

export const LicenseModalSideImage = ({
  variant,
}: {
  variant: LicenseModalSideImageVariantValue;
}) => {
  switch (variant) {
    case 'limit':
      return (
        <img
          src={limitImage}
          id="license-side-limit"
          width={256}
          height={463}
          style={{
            top: -12,
            left: -15,
          }}
        />
      );
    case 'business':
      return (
        <img
          src={businessImage}
          id="license-side-business"
          width={499}
          height={499}
          style={{
            left: -160,
            top: -20,
          }}
        />
      );
    case 'enterprise':
      return (
        <img
          src={enterpriseImage}
          id="license-side-enterprise"
          width={460}
          height={510}
          style={{
            left: -180,
            top: -40,
          }}
        />
      );
    case 'expired':
      return (
        <img
          src={expiredImage}
          id="license-side-expired"
          width={329}
          height={447}
          style={{
            left: 10,
            top: -10,
          }}
        />
      );
  }
};
