import clipboard from 'clipboardy';
import parse from 'html-react-parser';
import { useMemo } from 'react';
import QRCode from 'react-qr-code';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import {
  ActionButton,
  ActionButtonVariant,
} from '../../../../../../../shared/components/layout/ActionButton/ActionButton';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/components/layout/Button/Button';
import { ExpandableCard } from '../../../../../../../shared/components/layout/ExpandableCard/ExpandableCard';
import { Helper } from '../../../../../../../shared/components/layout/Helper/Helper';
import { Input } from '../../../../../../../shared/components/layout/Input/Input';
import { Label } from '../../../../../../../shared/components/layout/Label/Label';
import MessageBox, {
  MessageBoxType,
} from '../../../../../../../shared/components/layout/MessageBox/MessageBox';
import { useModalStore } from '../../../../../../../shared/hooks/store/useModalStore';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { downloadWGConfig } from '../../../../../../../shared/utils/downloadWGConfig';

export const ConfigStep = () => {
  const { LL, locale } = useI18nContext();
  const config = useModalStore((state) => state.userDeviceModal.config);
  const deviceName = useModalStore((state) => state.userDeviceModal.deviceName);
  const nextStep = useModalStore((state) => state.userDeviceModal.nextStep);
  const toaster = useToaster();

  const expandableCardActions = useMemo(() => {
    return [
      <ActionButton variant={ActionButtonVariant.QRCODE} key={1} forcedActive={true} />,
      <ActionButton
        variant={ActionButtonVariant.COPY}
        key={2}
        onClick={() => {
          if (config) {
            clipboard
              .write(config)
              .then(() => {
                toaster.success(
                  LL.modals.addDevice.web.steps.config.messages.copyConfig()
                );
              })
              .catch(() => {
                toaster.error(LL.messages.clipboardError());
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [config, deviceName, toaster, locale]);

  return (
    <>
      <MessageBox type={MessageBoxType.WARNING}>
        {parse(LL.modals.addDevice.web.steps.config.warningMessage())}
      </MessageBox>
      <Input
        outerLabel={LL.modals.addDevice.web.steps.config.inputNameLabel()}
        value={deviceName ?? ''}
        onChange={() => {
          return;
        }}
        disabled={true}
      />
      <div className="info">
        <p>{LL.modals.addDevice.web.steps.config.qrInfo()}</p>
      </div>
      <div className="card-label">
        <Label>{LL.modals.addDevice.web.steps.config.qrLabel()}</Label>
        <Helper initialPlacement="right">
          {parse(LL.modals.addDevice.web.steps.config.qrHelper())}
        </Helper>
      </div>
      {config && config.length > 0 && (
        <ExpandableCard
          title={LL.modals.addDevice.web.steps.config.qrCardTitle()}
          actions={expandableCardActions}
          expanded
        >
          <QRCode value={config} size={250} />
        </ExpandableCard>
      )}
      <div className="controls">
        <Button
          text={LL.form.close()}
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={() => nextStep()}
        />
      </div>
    </>
  );
};
