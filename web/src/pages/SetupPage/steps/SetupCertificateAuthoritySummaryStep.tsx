import { useQuery } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { downloadFile } from '../../../shared/utils/download';
import caIcon from '../assets/ca.png';
import { CAOption, SetupPageStep } from '../types';
import { useSetupWizardStore } from '../useSetupWizardStore';
import './style.scss';

export const SetupCertificateAuthoritySummaryStep = () => {
  const setActiveStep = useSetupWizardStore((s) => s.setActiveStep);
  const caOption = useSetupWizardStore((s) => s.ca_option);

  const { data: caData, isFetching } = useQuery({
    queryKey: ['initial_setup', 'ca'],
    queryFn: api.initial_setup.getCA,
    select: (resp) => resp.data,
  });

  const handleDownloadCA = useCallback(() => {
    const caPem = caData?.ca_cert_pem;
    if (!isPresent(caPem)) return;
    const blob = new Blob([caPem], {
      type: 'application/x-pem-file;charset=utf-8',
    });
    downloadFile(blob, 'defguard-ca', 'pem');
  }, [caData?.ca_cert_pem]);

  const handleBack = () => {
    setActiveStep(SetupPageStep.CertificateAuthority);
  };

  const handleNext = () => {
    setActiveStep(SetupPageStep.EdgeComponent);
  };

  const downloadCA = () => {
    return (
      <ActionCard
        title="Certificate Authority Generated"
        subtitle="The system created all required certificate files, including the root certificate and private key. You can download these files and continue with the configuration."
        imageSrc={caIcon}
      >
        <Button
          iconLeft="download"
          variant="outlined"
          text={`${m.controls_download()} CA certificate`}
          onClick={handleDownloadCA}
          loading={isFetching}
          disabled={!isPresent(caData?.ca_cert_pem) || isFetching}
        />
      </ActionCard>
    );
  };
  const getValidityString = (validForDays?: number) => {
    if (!validForDays) return '—';
    try {
      const years = Math.round(validForDays / 365);
      if (years <= 0) return 'Less than a year';
      return years === 1 ? '1 year' : `${years} years`;
    } catch (e) {
      console.error('Error calculating validity string:', e);
      return '—';
    }
  };

  const displayCAInfo = () => {
    if (!isPresent(caData)) return null;

    const commonName = caData.subject_common_name || '—';
    const validity = getValidityString(caData.valid_for_days);

    return (
      <ActionCard
        title={'Certificate Authority Validated'}
        subtitle={
          'Your uploaded Certificate Authority has been successfully validated. All required files were checked and confirmed as correct and ready for use. You can download the validated CA files if needed for your setup.'
        }
        imageSrc={caIcon}
      >
        <div className="ca-info">
          <p className="ca-info-title">Information extracted from uploaded file</p>
          <Divider spacing={ThemeSpacing.Md} />
          <div className="ca-info-grid">
            <div className="ca-info-label">Common Name</div>
            <div className="ca-info-value">{commonName}</div>

            <div className="ca-info-label">Validity</div>
            <div className="ca-info-value">{validity}</div>
          </div>
        </div>
      </ActionCard>
    );
  };

  return (
    <WizardCard>
      {caOption === CAOption.Create && downloadCA()}
      {caOption === CAOption.UseOwn && displayCAInfo()}
      <ModalControls
        cancelProps={{ text: 'Back', onClick: handleBack, variant: 'outlined' }}
        submitProps={{ text: 'Next', onClick: handleNext }}
      />
    </WizardCard>
  );
};
