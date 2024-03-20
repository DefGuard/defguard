import './style.scss';

import Skeleton from 'react-loading-skeleton';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { useAddDevicePageStore } from '../../../addDevice/hooks/useAddDevicePageStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { DeviceCard } from './DeviceCard/DeviceCard';
import { DeleteUserDeviceModal } from './modals/DeleteUserDeviceModal/DeleteUserDeviceModal';
import { DeviceConfigModal } from './modals/DeviceConfigModal/DeviceConfigModal';
import { EditUserDeviceModal } from './modals/EditUserDeviceModal/EditUserDeviceModal';

export const UserDevices = () => {
  const navigate = useNavigate();
  const { LL } = useI18nContext();
  const userProfile = useUserProfileStore((state) => state.userProfile);
  // const [initAddDevice, networks] = useAddDevicePageStore((state) => [
  const [initAddDevice, networks] = useAddDevicePageStore((state) => [
    state.init,
    state.networks,
  ]);

  // const [networks] = useAddDevicePageStore(
  //   (state) => [
  //     state.networks,
  //   ],
  //   shallow,
  // );

  console.log(networks, !networks?.length);
  return (
    <section id="user-devices">
      <header>
        <h2>{LL.userPage.devices.header()}</h2>
      </header>
      {!userProfile && (
        <div className="skeletons">
          <Skeleton />
          <Skeleton />
          <Skeleton />
        </div>
      )}
      {userProfile && (
        <>
          {userProfile.devices && userProfile.devices.length > 0 && (
            <div className="devices">
              {userProfile.devices.map((device) => (
                <DeviceCard key={device.id} device={device} />
              ))}
            </div>
          )}
          {userProfile && (
            <AddComponentBox
              data-testid="add-device"
              text={LL.userPage.devices.addDevice.web()}
              disabled={!networks?.length}
              callback={() => {
                initAddDevice({
                  username: userProfile.user.username,
                  id: userProfile.user.id,
                  reservedDevices: userProfile.devices.map((d) => d.name),
                  email: userProfile.user.email,
                  originRoutePath: window.location.pathname,
                });
                navigate('/add-device', { replace: true });
              }}
            />
          )}
        </>
      )}
      <DeleteUserDeviceModal />
      <EditUserDeviceModal />
      <DeviceConfigModal />
    </section>
  );
};
