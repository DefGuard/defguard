import { m } from '../../../../../../paraglide/messages';
import { Divider } from '../../../../../defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../../defguard-ui/components/ModalControls/ModalControls';
import { useAddUserDeviceModal } from '../../store/useAddUserDeviceModal';
import './style.scss';
import { isPresent } from '../../../../../defguard-ui/utils/isPresent';
import { ModalDeviceConfigSection } from '../../../../ModalDeviceConfigSection/ModalDeviceConfigSection';

export const AddDeviceModalManualDownloadStep = () => {
  const response = useAddUserDeviceModal((s) => s.createDeviceResponse);
  const keys = useAddUserDeviceModal((s) => s.manualConfig);
  if (!isPresent(response) || !isPresent(keys))
    throw new Error('Required store data not present during render');

  return (
    <div id="add-user-device-manual-download">
      <p>{m.modal_add_user_device_manual_download_warn_title()}</p>
      <p>{m.modal_add_user_device_manual_download_warn_content()}</p>
      <Divider orientation="horizontal" />
      <ModalDeviceConfigSection data={response} privateKey={keys.privateKey} />
      <Divider />
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
