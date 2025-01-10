import './style.scss';

import { ReactNode, useCallback, useMemo, useState } from 'react';
import QRCode from 'react-qr-code';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { RenderMarkdown } from '../../../../../../shared/components/Layout/RenderMarkdown/RenderMarkdown';
import { ActionButton } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useClipboard } from '../../../../../../shared/hooks/useClipboard';
import { downloadWGConfig } from '../../../../../../shared/utils/downloadWGConfig';
import { useAddStandaloneDeviceModal } from '../../store';

export const FinishManualStep = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.addStandaloneDevice.steps.manual.finish;
  const [closeModal] = useAddStandaloneDeviceModal((s) => [s.close], shallow);
  const manual = useAddStandaloneDeviceModal((s) => s.manualResponse);
  const generatedKeys = useAddStandaloneDeviceModal((s) => s.genKeys);

  return (
    <div className="finish-manual-step">
      <MessageBox
        type={MessageBoxType.INFO}
        message={localLL.messageTop()}
        dismissId="add-standalone-device-modal-manual-finish-header-info"
      />
      <div className="device-name">
        <p className="label">{LL.modals.addStandaloneDevice.form.labels.deviceName()}:</p>
        <p className="name">{manual?.device.name}</p>
      </div>
      <div className="cta">
        <p>{localLL.ctaInstruction()}</p>
      </div>
      <MessageBox type={MessageBoxType.ERROR}>
        <RenderMarkdown content={localLL.warningMessage()} />
      </MessageBox>
      {manual && (
        <DeviceConfigCard
          config={manual.config.config}
          publicKey={manual.config.pubkey}
          privateKey={generatedKeys?.privateKey}
          deviceName={manual.device.name}
        />
      )}
      <div className="controls solo">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.close()}
          onClick={() => {
            closeModal();
          }}
        />
      </div>
    </div>
  );
};

type ConfigCardProps = {
  config: string;
  privateKey?: string;
  publicKey: string;
  deviceName: string;
};

enum ConfigCardView {
  FILE,
  QR,
}

const DeviceConfigCard = ({
  config,
  privateKey,
  publicKey,
  deviceName,
}: ConfigCardProps) => {
  const { LL } = useI18nContext();
  const { writeToClipboard } = useClipboard();
  const localLL = LL.modals.addStandaloneDevice.steps.manual.finish;
  const [view, setView] = useState(ConfigCardView.FILE);

  const configForExport = useMemo(() => {
    if (privateKey) {
      return config.replace('YOUR_PRIVATE_KEY', privateKey);
    }
    return config;
  }, [config, privateKey]);

  const getQRConfig = useMemo((): string => {
    if (privateKey) {
      return config.replace('YOUR_PRIVATE_KEY', privateKey);
    }
    return config.replace('YOUR_PRIVATE_KEY', publicKey);
  }, [config, privateKey, publicKey]);

  const renderTextConfig = () => {
    const content = configForExport.split('\n');
    return (
      <p className="config">
        {content.map((text, index) => (
          <>
            <span>{text}</span>
            {index !== content.length - 1 && <br />}
          </>
        ))}
      </p>
    );
  };

  const handleConfigCopy = useCallback(() => {
    void writeToClipboard(
      configForExport,
      LL.components.deviceConfigsCard.messages.copyConfig(),
    );
  }, [LL.components.deviceConfigsCard.messages, configForExport, writeToClipboard]);

  const handleConfigDownload = useCallback(() => {
    downloadWGConfig(configForExport, deviceName.toLowerCase().replace(' ', '-'));
  }, [configForExport, deviceName]);

  const actions = useMemo(
    (): ReactNode[] => [
      <ActionButton
        variant={ActionButtonVariant.CONFIG}
        key={0}
        active={view === ConfigCardView.FILE}
        onClick={() => setView(ConfigCardView.FILE)}
      />,
      <ActionButton
        variant={ActionButtonVariant.QRCODE}
        key={1}
        active={view === ConfigCardView.QR}
        onClick={() => setView(ConfigCardView.QR)}
      />,
      <ActionButton
        variant={ActionButtonVariant.COPY}
        key={2}
        onClick={handleConfigCopy}
      />,
      <ActionButton
        variant={ActionButtonVariant.DOWNLOAD}
        key={3}
        onClick={handleConfigDownload}
      />,
    ],
    [handleConfigCopy, handleConfigDownload, view],
  );
  return (
    <ExpandableCard title={localLL.actionCard.title()} actions={actions} expanded={true}>
      {view === ConfigCardView.FILE && renderTextConfig()}
      {view === ConfigCardView.QR && <QRCode size={250} value={getQRConfig} />}
    </ExpandableCard>
  );
};
