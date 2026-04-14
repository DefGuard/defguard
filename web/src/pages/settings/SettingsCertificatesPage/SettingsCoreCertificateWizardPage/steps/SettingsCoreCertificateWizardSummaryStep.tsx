import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../../../paraglide/messages';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { useSettingsCoreCertificateWizardStore } from '../useSettingsCoreCertificateWizardStore';
import '../style.scss';

export const SettingsCoreCertificateWizardSummaryStep = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const handleFinish = async () => {
    await queryClient.invalidateQueries({ queryKey: ['core', 'cert', 'certs'] });
    await queryClient.invalidateQueries({ queryKey: ['core', 'cert', 'ca'] });
    useSettingsCoreCertificateWizardStore.getState().reset();
    await navigate({ to: '/settings/certs' });
  };

  return (
    <WizardCard className="settings-core-certificate-summary-card">
      <p className="summary-title">
        {m.settings_certs_core_wizard_summary_success_title()}
      </p>
      <SizedBox height={ThemeSpacing.Md} />
      <p className="summary-description">
        {m.settings_certs_core_wizard_summary_success_description()}
      </p>
      <SizedBox height={ThemeSpacing.Sm} />
      <p className="summary-description">
        {m.settings_certs_core_wizard_summary_url_notice()}
      </p>
      <Controls>
        <div className="right">
          <Button
            text={m.settings_certs_core_wizard_summary_ok()}
            onClick={handleFinish}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
