import './style.scss';

import { fs } from '@tauri-apps/api';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';

import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { DeviceCard } from './DeviceCard/DeviceCard';
import { AddDeviceModalDesktop } from './modals/AddDeviceModalDesktop/AddDeviceModalDesktop';
import { UserDeviceModal } from './modals/AddUserDeviceModal/AddUserDeviceModal';
import { DeleteUserDeviceModal } from './modals/DeleteUserDeviceModal/DeleteUserDeviceModal';
import { EditUserDeviceModal } from './modals/EditUserDeviceModal/EditUserDeviceModal';

export const UserDevices = () => {
  const isDesktopApp = useMemo(() => !isUndefined(window.__TAURI__), []);
  const isDeviceConfigPresent = useMemo(async () => {
    if (isDesktopApp) {
      const appDir = fs.BaseDirectory.AppData;
      return await fs.exists('wg/device.conf', { dir: appDir });
    }
    return false;
  }, [isDesktopApp]);
  const setModalsState = useModalStore((state) => state.setState);
  const user = useUserProfileStore((state) => state.user);
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
                currentStep: 0,
                config: undefined,
                deviceName: undefined,
              })
            }
          />
          {isDesktopApp && !isDeviceConfigPresent && (
            <AddComponentBox
              text="Add this device"
              callback={() => {
                setModalsState({ addDeviceDesktopModal: { visible: true } });
              }}
            />
          )}
        </>
      )}
      <AddDeviceModalDesktop />
      <DeleteUserDeviceModal />
      <EditUserDeviceModal />
      <UserDeviceModal />
    </section>
  );
};
