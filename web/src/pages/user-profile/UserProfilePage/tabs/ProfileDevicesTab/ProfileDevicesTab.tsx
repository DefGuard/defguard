import { LayoutGrid } from '../../../../../shared/components/LayoutGrid/LayoutGrid';
import './style.scss';
import { AddUserDeviceModal } from '../../../../../shared/components/modals/AddUserDeviceModal/AddUserDeviceModal';
import { AssignUserDeviceIPModal } from '../../../../../shared/components/modals/AssignUserDeviceIPModal/AssignUserDeviceIPModal';
import { EditUserDeviceModal } from '../../../../../shared/components/modals/EditUserDeviceModal/EditUserDeviceModal';
import { UserDeviceConfigModal } from '../../../../../shared/components/modals/UserDeviceConfigModal/UserDeviceConfigModal';
import { ProfileDevicesTable } from './components/ProfileDevicesTable/ProfileDevicesTable';

export const ProfileDevicesTab = () => {
  return (
    <>
      <LayoutGrid id="profile-devices-tab">
        <ProfileDevicesTable />
      </LayoutGrid>
      <AddUserDeviceModal />
      <EditUserDeviceModal />
      <UserDeviceConfigModal />
      <AssignUserDeviceIPModal />
    </>
  );
};
