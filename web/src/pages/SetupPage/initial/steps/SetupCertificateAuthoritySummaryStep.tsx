import { useQuery } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { ActionCard } from '../../../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { downloadFile } from '../../../../shared/utils/download';
import caIcon from '../../assets/ca.png';
import { SetupPageStep } from '../types';
import { useSetupWizardStore } from '../useSetupWizardStore';
import './style.scss';

export const SetupCertificateAuthoritySummaryStep = () => {
  const setActiveStep = useSetupWizardStore((s) => s.setActiveStep);

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
    setActiveStep(SetupPageStep.EdgeDeploy);
  };

  return (
    <WizardCard>
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
      <Controls>
        <Button
          text={m.initial_setup_controls_back()}
          onClick={handleBack}
          variant="outlined"
        />
        <div className="right">
          <Button text={m.initial_setup_controls_continue()} onClick={handleNext} />
        </div>
      </Controls>
    </WizardCard>
  );
};
