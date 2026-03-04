import { useQuery } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { ActionCard } from '../../../../shared/components/ActionCard/ActionCard';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { downloadFile } from '../../../../shared/utils/download';
import caIcon from '../../assets/ca.png';
import { CertificateAuthorityInfoCard } from '../components/CertificateAuthorityInfoCard';
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
        title={m.initial_setup_ca_generated_title()}
        subtitle={m.initial_setup_ca_generated_subtitle()}
        imageSrc={caIcon}
      >
        <Button
          iconLeft="download"
          variant="outlined"
          text={m.initial_setup_ca_download_button()}
          onClick={handleDownloadCA}
          loading={isFetching}
          disabled={!isPresent(caData?.ca_cert_pem) || isFetching}
        />
      </ActionCard>
    );
  };
  const getValidityString = (validForDays?: number) => {
    if (!validForDays) return m.initial_setup_ca_validity_unknown();
    try {
      const years = Math.round(validForDays / 365);
      if (years <= 0) return m.initial_setup_ca_validity_less_than_year();
      return years === 1
        ? m.initial_setup_ca_validity_one_year()
        : m.initial_setup_ca_validity_years({ years });
    } catch (e) {
      console.error('Error calculating validity string:', e);
      return m.initial_setup_ca_validity_unknown();
    }
  };

  const displayCAInfo = () => {
    if (!isPresent(caData)) return null;

    const commonName = caData.subject_common_name || '—';
    const validity = getValidityString(caData.valid_for_days);

    return (
      <CertificateAuthorityInfoCard
        title={m.initial_setup_ca_validated_title()}
        subtitle={m.initial_setup_ca_validated_subtitle()}
        infoTitle={m.initial_setup_ca_info_title()}
        commonNameLabel={m.initial_setup_ca_info_label_common_name()}
        validityLabel={m.initial_setup_ca_info_label_validity()}
        commonName={commonName}
        validity={validity}
        imageSrc={caIcon}
      />
    );
  };

  return (
    <WizardCard>
      {caOption === CAOption.Create && downloadCA()}
      {caOption === CAOption.UseOwn && displayCAInfo()}
      <ModalControls
        cancelProps={{
          text: m.initial_setup_controls_back(),
          onClick: handleBack,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.initial_setup_controls_continue(),
          onClick: handleNext,
        }}
      />
    </WizardCard>
  );
};
