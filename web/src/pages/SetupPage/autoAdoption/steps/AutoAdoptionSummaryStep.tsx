import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import {
  WizardStepSummary,
  type WizardStepSummaryRecommendation,
} from '../../../../shared/components/wizard-steps/WizardStepSummary/WizardStepSummary';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import CommunityIcon from '../../assets/community.png';
import FileIcon from '../../assets/file-icon.png';
import ShieldIcon from '../../assets/shield.png';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';

export const AutoAdoptionSummaryStep = () => {
  const navigate = useNavigate();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const wireguardPort = useAutoAdoptionSetupWizardStore((s) => s.vpn_wireguard_port);

  const waitForSettingsEssentials = async ({
    timeoutMs = 60_000,
    intervalMs = 500,
  }: {
    timeoutMs?: number;
    intervalMs?: number;
  }) => {
    const startedAt = Date.now();

    while (Date.now() - startedAt < timeoutMs) {
      try {
        const response = await api.getSessionInfo();

        if (isPresent(response.data) && response.data.active_wizard === null) {
          return;
        }
      } catch (_error) {
        // Ignore errors while API restarts.
      }

      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }

    throw new Error(m.initial_setup_auto_adoption_summary_error_settings_timeout());
  };

  const { mutateAsync: finishSetup } = useMutation({
    mutationKey: ['finish-setup'],
    mutationFn: api.initial_setup.finishSetup,
    meta: {
      invalidate: [['settings_essentials'], ['session-info']],
    },
  });

  const handleGoToDefguard = async () => {
    try {
      setIsSubmitting(true);
      await finishSetup();
      await waitForSettingsEssentials({});
      await navigate({ to: '/vpn-overview', replace: true });
      setTimeout(() => {
        useAutoAdoptionSetupWizardStore.getState().reset();
      }, 100);
    } catch (error) {
      console.error(m.initial_setup_auto_adoption_summary_error_finish_console(), error);
      Snackbar.error(m.initial_setup_confirmation_error_finish_failed());
    } finally {
      setIsSubmitting(false);
    }
  };

  const recommendations: WizardStepSummaryRecommendation[] = [
    {
      iconSrc: FileIcon,
      iconAlt: m.initial_setup_auto_adoption_summary_docs_icon_alt(),
      kicker: m.initial_setup_auto_adoption_summary_docs_kicker(),
      title: m.initial_setup_auto_adoption_summary_docs_title(),
      buttonText: m.initial_setup_auto_adoption_summary_docs_button(),
      onButtonClick: () => window.open('https://docs.defguard.net/', '_blank'),
    },
    {
      iconSrc: CommunityIcon,
      iconAlt: m.initial_setup_auto_adoption_summary_community_icon_alt(),
      kicker: m.initial_setup_auto_adoption_summary_community_kicker(),
      title: m.initial_setup_auto_adoption_summary_community_title(),
      buttonText: m.initial_setup_auto_adoption_summary_community_button(),
      onButtonClick: () =>
        window.open('https://github.com/DefGuard/defguard/discussions', '_blank'),
    },
    {
      iconSrc: ShieldIcon,
      iconAlt: m.initial_setup_auto_adoption_summary_support_icon_alt(),
      kicker: m.initial_setup_auto_adoption_summary_support_kicker(),
      title: m.initial_setup_auto_adoption_summary_support_title(),
      buttonText: m.initial_setup_auto_adoption_summary_support_button(),
      onButtonClick: () => window.open('https://github.com/DefGuard/defguard', '_blank'),
    },
  ];

  return (
    <WizardStepSummary
      thankYouText={m.initial_setup_auto_adoption_summary_thank_you()}
      noteText={m.initial_setup_auto_adoption_summary_note()}
      ports={[
        m.initial_setup_auto_adoption_summary_ports_http_https(),
        m.initial_setup_auto_adoption_summary_ports_wireguard({ port: wireguardPort }),
      ]}
      encourageText={m.initial_setup_auto_adoption_summary_encourage()}
      recommendations={recommendations}
      submitButtonText={m.initial_setup_auto_adoption_summary_submit()}
      onSubmit={handleGoToDefguard}
      submitLoading={isSubmitting}
      className="auto-adoption-summary-step"
    />
  );
};
