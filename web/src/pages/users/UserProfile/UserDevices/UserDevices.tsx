import './style.scss';

import { fs } from '@tauri-apps/api';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';
import { AddComponentBox } from '../../shared/components/AddComponentBox/AddComponentBox';
import { DeviceCard } from './DeviceCard/DeviceCard';
import { useDeviceModal } from './hooks/useDeviceModal';
import { AddDeviceModalDesktop } from './modals/AddDeviceModalDesktop/AddDeviceModalDesktop';
import { DeleteUserDeviceModal } from './modals/DeleteUserDeviceModal/DeleteUserDeviceModal';
import { EditUserDeviceModal } from './modals/EditUserDeviceModal/EditUserDeviceModal';
import { UserDeviceModal } from './modals/UserDeviceModal/UserDeviceModal';

export const UserDevices = () => {
  const appInfo = useAppStore((state) => state.appInfo);
  const { LL } = useI18nContext();
  const isDesktopApp = useMemo(() => !isUndefined(window.__TAURI__), []);
  const isDeviceConfigPresent = useMemo(async () => {
    if (isDesktopApp) {
      const appDir = fs.BaseDirectory.AppData;
      return await fs.exists('wg/device.conf', { dir: appDir });
    }
    return false;
  }, [isDesktopApp]);
  const setModalsState = useModalStore((state) => state.setState);
  const userProfile = useUserProfileStore((state) => state.userProfile);
  const openDeviceModal = useDeviceModal((state) => state.open);

  return (
    <section id="user-devices">
      <header>
        <h2>{LL.userPage.devices.header()}</h2>
      </header>
      {userProfile && (
        <>
          {userProfile.devices && userProfile.devices.length > 0 && (
            <div className="devices">
              {userProfile.devices.map((device) => (
                <DeviceCard key={device.id} device={device} />
              ))}
            </div>
          )}
          <AddComponentBox
            data-testid="add-device"
            text={LL.userPage.devices.addDevice.web()}
            disabled={!appInfo?.network_present}
            callback={() =>
              openDeviceModal({
                visible: true,
              })
            }
          />
          {isDesktopApp && !isDeviceConfigPresent && (
            <AddComponentBox
              disabled={!appInfo?.network_present}
              text={LL.userPage.devices.addDevice.desktop()}
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
