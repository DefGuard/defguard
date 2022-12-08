import QRCode from 'react-qr-code';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import { Helper } from '../../../../../../shared/components/layout/Helper/Helper';
import { Input } from '../../../../../../shared/components/layout/Input/Input';
import { Label } from '../../../../../../shared/components/layout/Label/Label';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';

export const ConfigStep = () => {
  const config = useModalStore((state) => state.userDeviceModal.config);
  const deviceName = useModalStore((state) => state.userDeviceModal.deviceName);
  const nextStep = useModalStore((state) => state.userDeviceModal.nextStep);
  const setupChoice = useModalStore((state) => state.userDeviceModal.choice);

  return (
    <>
      <p>{setupChoice?.valueOf()}</p>
      <MessageBox type={MessageBoxType.WARNING}>
        <p>
          Please be advised that you have to download the configuration now,
          since <strong>we do not</strong> store your private key. After this
          dialog is closed, you <strong>will not be able</strong> to get your
          full configuration file (with private keys, only blank template).
        </p>
      </MessageBox>
      <Input
        outerLabel="Device Name"
        value={deviceName}
        onChange={() => {
          return;
        }}
        disabled={true}
      />
      <p className="info">
        Use provided configuration file below by scanning QR Code or importing
        it as file on your devices WireGuard instance.
      </p>
      <div className="card-label">
        <Label>WireGuard Config File:</Label>
        <Helper initialPlacement="right">
          <p>
            This configuration file can be scanned, copied or downloaded, but
            needs to be used
            <strong>on your device that you are adding now.</strong>
            <a>Read more in documentation.</a>
          </p>
        </Helper>
      </div>
      {config && <QRCode value={config} size={250} />}
      <div className="controls">
        <Button
          text="Close"
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => nextStep()}
        />
      </div>
    </>
  );
};
