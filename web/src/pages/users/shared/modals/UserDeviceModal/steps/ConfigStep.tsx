import clipboard from 'clipboardy';
import { useMemo } from 'react';
import QRCode from 'react-qr-code';

import {
  ActionButton,
  ActionButtonVariant,
} from '../../../../../../shared/components/layout/ActionButton/ActionButton';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import { ExpandableCard } from '../../../../../../shared/components/layout/ExpandableCard/ExpandableCard';
import { Helper } from '../../../../../../shared/components/layout/Helper/Helper';
import { Input } from '../../../../../../shared/components/layout/Input/Input';
import { Label } from '../../../../../../shared/components/layout/Label/Label';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../shared/components/layout/MessageBox/MessageBox';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { downloadWGConfig } from '../../../../../../shared/utils/downloadWGConfig';

export const ConfigStep = () => {
  const config = useModalStore((state) => state.userDeviceModal.config);
  const deviceName = useModalStore((state) => state.userDeviceModal.deviceName);
  const nextStep = useModalStore((state) => state.userDeviceModal.nextStep);
  const toaster = useToaster();

  const expandableCardActions = useMemo(() => {
    return [
      <ActionButton
        variant={ActionButtonVariant.QRCODE}
        key={1}
        forcedActive={true}
      />,
      <ActionButton
        variant={ActionButtonVariant.COPY}
        key={2}
        onClick={() => {
          if (config) {
            clipboard
              .write(config)
              .then(() => {
                toaster.success('Config copied to clipboard.');
              })
              .catch(() => {
                toaster.error(
                  'Clipboard is not available.',
                  'Make sure you are in secure context.'
                );
              });
          }
        }}
      />,
      <ActionButton
        variant={ActionButtonVariant.DOWNLOAD}
        key={3}
        onClick={() => {
          if (config && deviceName) {
            downloadWGConfig(config, deviceName);
          }
        }}
      />,
    ];
  }, [config, deviceName, toaster]);

  return (
    <>
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
        value={deviceName ?? ''}
        onChange={() => {
          return;
        }}
        disabled={true}
      />
      <div className="info">
        <p>
          Use provided configuration file below by scanning QR Code or importing
          it as file on your devices WireGuard instance.
        </p>
      </div>
      <div className="card-label">
        <Label>WireGuard Config File</Label>
        <Helper initialPlacement="right">
          <p>
            This configuration file can be scanned, copied or downloaded, but
            needs to be used
            <strong>on your device that you are adding now.</strong>
            <a>Read more in documentation.</a>
          </p>
        </Helper>
      </div>
      {config && config.length > 0 && (
        <ExpandableCard
          title="WireGuard Config"
          actions={expandableCardActions}
          expanded
        >
          <QRCode value={config} size={250} />
        </ExpandableCard>
      )}
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
