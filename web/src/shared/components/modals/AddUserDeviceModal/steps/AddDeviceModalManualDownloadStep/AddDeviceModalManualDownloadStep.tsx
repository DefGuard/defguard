import { m } from '../../../../../../paraglide/messages';
import { ModalControls } from '../../../../../defguard-ui/components/ModalControls/ModalControls';
import { useAddUserDeviceModal } from '../../store/useAddUserDeviceModal';
import './style.scss';

export const AddDeviceModalManualDownloadStep = () => {
  return (
    <div className="add-user-device-manual-download">
      <ModalControls
        submitProps={{
          text: m.controls_complete(),
          onClick: () => {
            useAddUserDeviceModal.getState().close();
          },
        }}
      />
    </div>
  );
};
