import { useQuery } from '@tanstack/react-query';
import { saveAs } from 'file-saver';
import { useEffect } from 'react';
import { ReactMarkdown } from 'react-markdown/lib/react-markdown';

import { useI18nContext } from '../../../i18n/i18n-react';
import { Button } from '../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/types';
import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';
import SvgIconArrowGrayUp from '../../../shared/components/svg/IconArrowGrayUp';
import SvgIconDownload from '../../../shared/components/svg/IconDownload';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';

export const DebugDataCard = () => {
  const { LL } = useI18nContext();
  const settings = useAppStore((state) => state.settings);
  const smtp_configured =
    settings?.smtp_server &&
    settings?.smtp_port &&
    settings?.smtp_user &&
    settings?.smtp_password &&
    settings?.smtp_sender;
  const {
    settings: { downloadSupportData, downloadLogs },
    mail: {sendSupportMail},
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
  const sendMail = () => {
    // TODO
    console.log('send mail');
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
        onClick={sendMail}
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.PRIMARY}
        icon={<SvgIconArrowGrayUp />}
        text={LL.settingsPage.debugDataCard.sendMail()}
        disabled={!smtp_configured}
      />
    </>
  );
};
