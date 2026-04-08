import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { InternalSslResult } from '../../../../shared/components/certificates/InternalSslResult/InternalSslResult';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Layout/Divider/Divider';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { downloadFile } from '../../../../shared/utils/download';
import caIcon from '../../assets/ca.png';
import { AutoAdoptionSetupStep } from '../types';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';
import './style.scss';

export const AutoAdoptionInternalUrlSslConfigStep = () => {
  const setActiveStep = useAutoAdoptionSetupWizardStore((s) => s.setActiveStep);
  const sslType = useAutoAdoptionSetupWizardStore((s) => s.internal_ssl_type);
  const certInfo = useAutoAdoptionSetupWizardStore((s) => s.internal_ssl_cert_info);
  // If ssl_type is not set (e.g. fresh browser session), redirect back so the
  // user can re-submit the settings step and repopulate the store.
  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    if (sslType === null) {
      setActiveStep(AutoAdoptionSetupStep.InternalUrlSettings);
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
      <Divider />
      <Controls>
        <Button
          text={m.initial_setup_controls_back()}
          variant="outlined"
          onClick={() => setActiveStep(AutoAdoptionSetupStep.InternalUrlSettings)}
        />
        <div className="right">
          <Button
            text={m.initial_setup_controls_continue()}
            onClick={() => setActiveStep(AutoAdoptionSetupStep.ExternalUrlSettings)}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
