import './style.scss';
import { Link } from '@tanstack/react-router';
import { Fragment, useMemo } from 'react';
import { m } from '../../../../../../../../../paraglide/messages';
import { Divider } from '../../../../../../../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../../../../../../../shared/defguard-ui/components/Icon';
import { ThemeSpacing } from '../../../../../../../../../shared/defguard-ui/types';
import { useUserProfile } from '../../../../../../hooks/useUserProfilePage';
import { UserProfileTab } from '../../../../../types';

export const BiometricDevices = () => {
  const devices = useUserProfile((s) => s.devices);
  const biometricDevices = useMemo(
    () => devices.filter((device) => device.biometry_enabled),
    [devices],
  );

  if (biometricDevices.length === 0) return null;

  return (
    <>
      <Divider spacing={ThemeSpacing.Xl} />
      <div className="biometric-devices section">
        <div className="top">
          <p className="section-title">{m.profile_auth_card_biometric_title()}</p>
          <Link
            className="text-action"
            from="/user/$username"
            to="/user/$username"
            search={{
              tab: UserProfileTab.Devices,
            }}
          >
            {m.profile_auth_card_devices_link()}
          </Link>
        </div>
        {biometricDevices.map((device, index) => (
          <Fragment key={device.id}>
            <div className="device">
              <Icon icon="biometric" />
              <p>{device.name}</p>
            </div>
            {index !== biometricDevices.length - 1 && (
              <Divider spacing={ThemeSpacing.Lg} />
            )}
          </Fragment>
        ))}
      </div>
    </>
  );
};
