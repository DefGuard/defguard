import './style.scss';

import { Fragment, ReactNode, useCallback, useMemo, useState } from 'react';
import QRCode from 'react-qr-code';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ActionButton } from '../../../defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../defguard-ui/components/Layout/ActionButton/types';
import { ExpandableCard } from '../../../defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { useClipboard } from '../../../hooks/useClipboard';
import { downloadWGConfig } from '../../../utils/downloadWGConfig';

type Props = {
  config: string;
  publicKey: string;
  deviceName: string;
  privateKey?: string;
};

enum ConfigCardView {
  FILE,
  QR,
}

export const WireguardConfigExpandable = ({
  config,
  deviceName,
  publicKey,
  privateKey,
}: Props) => {
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
          <Fragment key={index}>
            <span>{text}</span>
            {index !== content.length - 1 && <br />}
          </Fragment>
        ))}
      </p>
    );
  };

  const handleConfigCopy = useCallback(() => {
    writeToClipboard(
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
    <ExpandableCard
      className="wireguard-config-card"
      title={localLL.actionCard.title()}
      actions={actions}
      expanded={true}
    >
      {view === ConfigCardView.FILE && renderTextConfig()}
      {view === ConfigCardView.QR && <QRCode size={250} value={getQRConfig} />}
    </ExpandableCard>
  );
};
