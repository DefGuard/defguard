import './style.scss';

import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';
import AddComponentBox from '../../shared/components/AddComponentBox/AddComponentBox';
import { DeviceCard } from './DeviceCard/DeviceCard';

export const UserDevices = () => {
  const user = useUserProfileV2Store((state) => state.user);
  const setUserDeviceModalState = useModalStore(
    (state) => state.setUserDeviceModal
  );
  return (
    <section id="user-devices">
      <header>
        <h2>User devices</h2>
      </header>
      {user && (
        <>
          {user.devices && user.devices.length > 0 && (
            <div className="devices">
              {user.devices.map((device) => (
                <DeviceCard key={device.id} device={device} />
              ))}
            </div>
          )}
          <AddComponentBox
            text="Add new device"
            callback={() =>
              setUserDeviceModalState({
                visible: true,
                device: undefined,
                username: user.username,
              })
            }
          />
        </>
      )}
    </section>
  );
};
