import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { InternalSslResult } from '../../../shared/components/certificates/InternalSslResult/InternalSslResult';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { downloadFile } from '../../../shared/utils/download';
import caIcon from '../../SetupPage/assets/ca.png';
import '../../SetupPage/autoAdoption/steps/style.scss';
import { SettingsCoreCertificateWizardStep } from '../types';
import { useSettingsCoreCertificateWizardStore } from '../useSettingsCoreCertificateWizardStore';

export const SettingsCoreCertificateWizardInternalUrlSslConfigStep = () => {
  const sslType = useSettingsCoreCertificateWizardStore((s) => s.internal_ssl_type);
  const certInfo = useSettingsCoreCertificateWizardStore((s) => s.internal_ssl_cert_info);

  useEffect(() => {
    if (sslType === null) {
      useSettingsCoreCertificateWizardStore.setState({
        activeStep: SettingsCoreCertificateWizardStep.InternalUrlSettings,
      });
    }
  }, [sslType]);

  const { data: caData } = useQuery({
    queryKey: ['core', 'cert', 'ca'],
    queryFn: api.core.getCA,
    enabled: sslType === 'defguard_ca',
    select: (response) => response.data,
  });

  const handleDownloadCaCert = () => {
    if (!caData?.ca_cert_pem) return;
    downloadFile(
      new Blob([caData.ca_cert_pem], { type: 'application/x-pem-file' }),
      'defguard-ca',
      'pem',
    );
  };

  return (
    <WizardCard>
      <InternalSslResult
        sslType={sslType}
        certInfo={certInfo}
        caCertPem={caData?.ca_cert_pem}
        onDownloadCaCert={handleDownloadCaCert}
        imageSrc={caIcon}
      />
      <SizedBox height={ThemeSpacing.Xl3} />
      <Divider />
      <Controls>
        <Button
          text={m.controls_back()}
          variant="outlined"
          onClick={() => useSettingsCoreCertificateWizardStore.getState().back()}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            onClick={() => useSettingsCoreCertificateWizardStore.getState().next()}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
