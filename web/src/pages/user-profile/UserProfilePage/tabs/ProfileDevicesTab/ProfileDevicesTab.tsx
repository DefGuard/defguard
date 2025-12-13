import { m } from '../../../../../paraglide/messages';
import { LayoutGrid } from '../../../../../shared/components/LayoutGrid/LayoutGrid';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { ProfileTabHeader } from '../../components/ProfileTabHeader/ProfileTabHeader';
import './style.scss';
import { AddUserDeviceModal } from '../../../../../shared/components/modals/AddUserDeviceModal/AddUserDeviceModal';
import { useAddUserDeviceModal } from '../../../../../shared/components/modals/AddUserDeviceModal/store/useAddUserDeviceModal';
import { EditUserDeviceModal } from '../../../../../shared/components/modals/EditUserDeviceModal/EditUserDeviceModal';
import { UserDeviceConfigModal } from '../../../../../shared/components/modals/UserDeviceConfigModal/UserDeviceConfigModal';
import { useUserProfile } from '../../hooks/useUserProfilePage';
import { ProfileDevicesTable } from './components/ProfileDevicesTable/ProfileDevicesTable';

export const ProfileDevicesTab = () => {
  const devices = useUserProfile((s) => s.devices);
  const user = useUserProfile((s) => s.user);

  return (
    <>
      <LayoutGrid id="profile-devices-tab">
        <SizedBox height={ThemeSpacing.Xl3} />
        <ProfileTabHeader title={m.profile_devices_title()}>
          <Button
            iconLeft="add-device"
            testId="add-device"
            text={m.profile_devices_add_new()}
            onClick={() => {
              useAddUserDeviceModal.getState().open({
                user,
                devices,
              });
            }}
          />
        </ProfileTabHeader>
        <ProfileDevicesTable />
      </LayoutGrid>
      <AddUserDeviceModal />
      <EditUserDeviceModal />
      <UserDeviceConfigModal />
    </>
  );
};
