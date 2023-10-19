import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { saveAs } from 'file-saver';
import { useState } from 'react';
import { ReactMarkdown } from 'react-markdown/lib/react-markdown';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgIconDownload from '../../../../shared/components/svg/IconDownload';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Divider } from '../../../../shared/defguard-ui/components/Layout/Divider/Divider';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../shared/hooks/useApi';
import { SendSupportDataModal } from './components/SendSupportDataModal';

export const DebugDataCard = () => {
  const [sendDataOpen, setSendDataOpen] = useState(false);
  const { LL } = useI18nContext();
  const settings = useAppStore((state) => state.settings);
  const smtp_configured =
    settings?.smtp_server &&
    settings?.smtp_port &&
    settings?.smtp_user &&
    settings?.smtp_password &&
    settings?.smtp_sender;

  const {
    support: { downloadSupportData, downloadLogs },
  } = useApi();

  const { isLoading: logsLoading, mutate: logsMutate } = useMutation({
    mutationFn: downloadLogs,
    onSuccess: (res) => {
      const content = new Blob([res], { type: 'text/plain;charset=utf-8' });
      const timestamp = new Date().toISOString().replaceAll(':', '');
      saveAs(content, `defguard-logs-${timestamp}.json`);
    },
  });

  const { isLoading: configLoading, mutate: configMutate } = useMutation({
    mutationFn: downloadSupportData,
    onSuccess: (res) => {
      const content = new Blob([JSON.stringify(res, null, 2)], {
        type: 'text/plain;charset=utf-8',
      });
      const timestamp = new Date().toISOString().replaceAll(':', '');
      saveAs(content, `defguard-support-data-${timestamp}.json`);
    },
  });

  return (
    <>
      <SendSupportDataModal
        isOpen={sendDataOpen}
        onOpenChange={(v) => setSendDataOpen(v)}
      />
      <Card id="support-debug-card" shaded bordered>
        <div className="controls">
          <p className="title">{LL.supportPage.debugDataCard.title()}</p>
          <Button
            onClick={() => {
              if (!configLoading) {
                configMutate();
              }
            }}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconDownload />}
            text={LL.supportPage.debugDataCard.downloadSupportData()}
            loading={configLoading}
          />
          <Button
            onClick={() => {
              if (!logsLoading) {
                logsMutate();
              }
            }}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={<SvgIconDownload />}
            text={LL.supportPage.debugDataCard.downloadLogs()}
            loading={logsLoading}
          />
          <Button
            onClick={() => {
              setSendDataOpen(true);
            }}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={LL.supportPage.debugDataCard.sendMail()}
            disabled={!smtp_configured}
          />
        </div>
        <Divider />
        <div className="content">
          <ReactMarkdown>{LL.supportPage.debugDataCard.body()}</ReactMarkdown>
        </div>
      </Card>
    </>
  );
};
