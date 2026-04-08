import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { InternalSslResult } from '../../../shared/components/certificates/InternalSslResult/InternalSslResult';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { downloadFile } from '../../../shared/utils/download';
import caIcon from '../../SetupPage/assets/ca.png';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';
import '../../SetupPage/autoAdoption/steps/style.scss';

export const MigrationWizardInternalUrlSslConfigStep = () => {
  const sslType = useMigrationWizardStore((s) => s.internal_ssl_type);
  const certInfo = useMigrationWizardStore((s) => s.internal_ssl_cert_info);

  // If ssl_type is not set (e.g. fresh browser session), redirect back so the
  // user can re-submit the settings step and repopulate the store.
  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    if (sslType === null) {
      useMigrationWizardStore.getState().back();
    }
  }, []);

  const { data: sslInfoData } = useQuery({
    queryKey: ['internal_ssl_info'],
    queryFn: () => api.initial_setup.getInternalSslInfo(),
    enabled: sslType === 'defguard_ca',
    select: (response) => response.data,
  });

  const handleDownloadCaCert = () => {
    if (!sslInfoData?.ca_cert_pem) return;
    downloadFile(
      new Blob([sslInfoData.ca_cert_pem], { type: 'application/x-pem-file' }),
      'defguard-ca',
      'pem',
    );
  };

  return (
    <WizardCard>
      <InternalSslResult
        sslType={sslType}
        certInfo={certInfo}
        caCertPem={sslInfoData?.ca_cert_pem}
        onDownloadCaCert={handleDownloadCaCert}
        imageSrc={caIcon}
      />
      <SizedBox height={ThemeSpacing.Xl3} />
      <Controls>
        <Button
          text={m.controls_back()}
          variant="outlined"
          onClick={() => useMigrationWizardStore.getState().back()}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            onClick={() => useMigrationWizardStore.getState().next()}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
