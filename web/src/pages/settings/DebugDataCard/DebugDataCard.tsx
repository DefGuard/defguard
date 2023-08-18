import { useMutation, useQuery } from '@tanstack/react-query';
import { saveAs } from 'file-saver';
import { useEffect } from 'react';
import { ReactMarkdown } from 'react-markdown/lib/react-markdown';

import { useI18nContext } from '../../../i18n/i18n-react';
import SvgIconArrowGrayUp from '../../../shared/components/svg/IconArrowGrayUp';
import SvgIconDownload from '../../../shared/components/svg/IconDownload';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { ContentCard } from '../../../shared/defguard-ui/components/Layout/ContentCard/ContentCard';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../shared/queries';
import { SMTPError } from '../../../shared/types';

export const DebugDataCard = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const settings = useAppStore((state) => state.settings);
  const smtp_configured =
    settings?.smtp_server &&
    settings?.smtp_port &&
    settings?.smtp_user &&
    settings?.smtp_password &&
    settings?.smtp_sender;
  const {
    support: { downloadSupportData, downloadLogs },
    mail: { sendSupportMail },
  } = useApi();
  const {
    data: supportData,
    isLoading: configLoading,
    refetch: fetchConfig,
  } = useQuery({
    queryKey: [QueryKeys.FETCH_SUPPORT_DATA],
    queryFn: downloadSupportData,
    enabled: false,
  });
  const {
    data: logs,
    isLoading: logsLoading,
    refetch: fetchLogs,
  } = useQuery({
    queryKey: [QueryKeys.FETCH_LOGS],
    queryFn: downloadLogs,
    enabled: false,
  });
  const { mutate: sendMail, isLoading: mailLoading } = useMutation([], sendSupportMail, {
    onSuccess: () => {
      toaster.success(LL.settingsPage.debugDataCard.mailSent());
    },
    onError: (err: SMTPError) => {
      toaster.error(
        `${LL.settingsPage.debugDataCard.mailError()}`,
        `${err.response?.data.error}`,
      );
      console.error(err);
    },
  });
  useEffect(() => {
    if (!supportData || configLoading) {
      return;
    }
    const content = new Blob([JSON.stringify(supportData, null, 2)], {
      type: 'text/plain;charset=utf-8',
    });
    const timestamp = new Date().toISOString().replaceAll(':', '');
    saveAs(content, `defguard-support-data-${timestamp}.json`);
  }, [supportData, configLoading]);

  useEffect(() => {
    if (!logs || logsLoading) {
      return;
    }
    const content = new Blob([logs], { type: 'text/plain;charset=utf-8' });
    const timestamp = new Date().toISOString().replaceAll(':', '');
    saveAs(content, `defguard-logs-${timestamp}.json`);
  }, [logs, logsLoading]);

  const onSendMail = async () => {
    sendMail();
  };

  return (
    <>
      <ContentCard
        header={<h3>{LL.settingsPage.debugDataCard.title()}</h3>}
        className="support"
      >
        <ReactMarkdown>{LL.settingsPage.debugDataCard.body()}</ReactMarkdown>
      </ContentCard>
      <Button
        className="support-data-button"
        onClick={() => fetchConfig()}
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.PRIMARY}
        icon={<SvgIconDownload />}
        text={LL.settingsPage.debugDataCard.downloadSupportData()}
      />
      <Button
        className="support-data-button"
        onClick={() => fetchLogs()}
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.PRIMARY}
        icon={<SvgIconDownload />}
        text={LL.settingsPage.debugDataCard.downloadLogs()}
      />
      <Button
        className="support-data-button"
        onClick={() => onSendMail()}
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.PRIMARY}
        icon={<SvgIconArrowGrayUp />}
        text={LL.settingsPage.debugDataCard.sendMail()}
        loading={mailLoading}
        disabled={!smtp_configured}
      />
    </>
  );
};
