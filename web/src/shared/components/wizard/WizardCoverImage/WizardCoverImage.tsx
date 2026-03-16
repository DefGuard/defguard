import edgeImage from './assets/edge_wizard_cover.png';
import gatewayImage from './assets/gw_wizard_cover.png';
import locationImage from './assets/location_wizard_cover.png';
import migrationImage from './assets/migration_wizard_cover.png';
import type { WizardCoverValue } from './types';

type Props = {
  variant: WizardCoverValue;
};

export const WizardCoverImage = ({ variant }: Props) => {
  switch (variant) {
    case 'edge':
      return (
        <img
          src={edgeImage}
          id="edge-wizard-cover"
          width={569}
          height={785}
          style={{
            top: -25,
            left: -30,
          }}
        />
      );
    case 'gateway':
      return (
        <img
          src={gatewayImage}
          id="gw-wizard-cover"
          width={794}
          height={1095}
          style={{
            top: -175,
          }}
        />
      );
    case 'location':
      return (
        <img
          src={locationImage}
          id="location-wizard-cover"
          width={540}
          height={855}
          style={{
            top: -130,
            left: -60,
          }}
        />
      );
    case 'migration':
      return (
        <img
          src={migrationImage}
          id="migration-wizard-cover"
          width={921}
          height={951}
          style={{
            top: -65,
            left: -268,
          }}
        />
      );
  }
};
