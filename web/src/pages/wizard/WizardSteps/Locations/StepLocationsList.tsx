import React from 'react';

import Badge from '../../../../shared/components/layout/Badge/Badge';
import { DeviceAvatar } from '../../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import MessageBox from '../../../../shared/components/layout/MessageBox/MessageBox';
import SvgIconPopupClose from '../../../../shared/components/svg/IconPopupClose';
import { useWizardStore } from '../store';
const StepLocationsList: React.FC = () => {
  const locations = useWizardStore((state) => state.locations);
  const removeLocation = useWizardStore((state) => state.removeLocation);

  return (
    <div className="locations-list">
      <h2>Locations:</h2>
      {locations.length ? (
        <ul>
          {locations.map((location) => (
            <li key={location.name}>
              <div className="location-icon">
                <DeviceAvatar active />
              </div>
              <div className="info">
                <h3>{location.name}</h3>
                <div className="badges">
                  <Badge text={location.ipAddress} />
                  {location.shared && location.shared.length ? (
                    <Badge text={location.shared.length.toString()} />
                  ) : null}
                </div>
              </div>
              <button
                className="icon-button"
                onClick={() => removeLocation(location)}
                data-test="delete-user"
              >
                <SvgIconPopupClose />
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <MessageBox message="You need to add at least one location." />
      )}
    </div>
  );
};

export default StepLocationsList;
