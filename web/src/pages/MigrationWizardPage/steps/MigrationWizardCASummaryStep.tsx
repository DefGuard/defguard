import { useQuery } from '@tanstack/react-query';
import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { downloadFile } from '../../../shared/utils/download';
import caIcon from '../../SetupPage/assets/ca.png';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

export const MigrationWizardCASummaryStep = () => {
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

  return (
    <WizardCard>
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
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useMigrationWizardStore.getState().back();
          }}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            onClick={() => {
              useMigrationWizardStore.getState().next();
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
