import { useQuery } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { downloadFile } from '../../../shared/utils/download';
import caIcon from '../../SetupPage/assets/ca.png';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';
import { CAOption, MigrationWizardStep } from '../types';

export const MigrationWizardCASummaryStep = () => {
  const setActiveStep = useMigrationWizardStore((s) => s.setActiveStep);
  const caOption = useMigrationWizardStore((s) => s.ca_option);

  const { data: caData, isFetching } = useQuery({
    queryKey: ['migration', 'ca'],
    queryFn: api.migration.ca.getCA,
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
    setActiveStep(MigrationWizardStep.Ca);
  };

  const handleNext = () => {
    setActiveStep(MigrationWizardStep.Edge);
  };

  const downloadCA = () => {
    return (
      <ActionCard
        title={m.migration_wizard_ca_generated_title()}
        subtitle={m.migration_wizard_ca_generated_subtitle()}
        imageSrc={caIcon}
      >
        <Button
          iconLeft="download"
          variant="outlined"
          text={m.migration_wizard_ca_download_button()}
          onClick={handleDownloadCA}
          loading={isFetching}
          disabled={!isPresent(caData?.ca_cert_pem) || isFetching}
        />
      </ActionCard>
    );
  };

  const getValidityString = (validForDays?: number) => {
    if (!validForDays) return m.migration_wizard_ca_validity_unknown();
    try {
      const years = Math.round(validForDays / 365);
      if (years <= 0) return m.migration_wizard_ca_validity_less_than_year();
      return years === 1
        ? m.migration_wizard_ca_validity_one_year()
        : m.migration_wizard_ca_validity_years({ years });
    } catch (e) {
      console.error('Error calculating validity string:', e);
      return m.migration_wizard_ca_validity_unknown();
    }
  };

  const displayCAInfo = () => {
    if (!isPresent(caData)) return null;

    const commonName = caData.subject_common_name || '—';
    const validity = getValidityString(caData.valid_for_days);

    return (
      <ActionCard
        title={m.migration_wizard_ca_validated_title()}
        subtitle={m.migration_wizard_ca_validated_subtitle()}
        imageSrc={caIcon}
      >
        <div className="ca-info">
          <p className="ca-info-title">{m.migration_wizard_ca_info_title()}</p>
          <Divider spacing={ThemeSpacing.Md} />
          <div className="ca-info-grid">
            <div className="ca-info-label">
              {m.migration_wizard_ca_info_label_common_name()}
            </div>
            <div className="ca-info-value">{commonName}</div>
            <div className="ca-info-label">
              {m.migration_wizard_ca_info_label_validity()}
            </div>
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
      <Controls>
        <Button variant="outlined" text={m.controls_back()} onClick={handleBack} />
        <div className="right">
          <Button text={m.controls_continue()} onClick={handleNext} />
        </div>
      </Controls>
    </WizardCard>
  );
};
