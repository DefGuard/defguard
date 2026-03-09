import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';
import './style.scss';

import CommunityIcon from '../../assets/community.png';
import FileIcon from '../../assets/file-icon.png';
import ShieldIcon from '../../assets/shield.png';

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

  return (
    <WizardCard className="auto-adoption-summary-step">
      <p className="thank-you">{m.initial_setup_auto_adoption_summary_thank_you()}</p>
      <Divider spacing={ThemeSpacing.Xl} />
      <p className="note">{m.initial_setup_auto_adoption_summary_note()}</p>
      <SizedBox height={ThemeSpacing.Lg} />
      <ul>
        <li>{m.initial_setup_auto_adoption_summary_ports_http_https()}</li>
        <li>
          {m.initial_setup_auto_adoption_summary_ports_wireguard({ port: wireguardPort })}
        </li>
      </ul>
      <Divider spacing={ThemeSpacing.Xl2} />
      <p className="encourage">{m.initial_setup_auto_adoption_summary_encourage()}</p>
      <SizedBox height={ThemeSpacing.Md} />
      <div className="recommendations">
        <div className="container">
          <img
            src={FileIcon}
            alt={m.initial_setup_auto_adoption_summary_docs_icon_alt()}
            className="icon"
          />
          <div className="recommendation-row">
            <div className="kicker-title">
              <p className="kicker">
                {m.initial_setup_auto_adoption_summary_docs_kicker()}
              </p>
              <p className="title">
                {m.initial_setup_auto_adoption_summary_docs_title()}
              </p>
            </div>
            <Button
              variant="outlined"
              text={m.initial_setup_auto_adoption_summary_docs_button()}
              iconRight="open-in-new-window"
              onClick={() => window.open('https://docs.defguard.net/', '_blank')}
            />
          </div>
        </div>

        <div className="container">
          <img
            src={CommunityIcon}
            alt={m.initial_setup_auto_adoption_summary_community_icon_alt()}
            className="icon"
          />
          <div className="recommendation-row">
            <div className="kicker-title">
              <p className="kicker">
                {m.initial_setup_auto_adoption_summary_community_kicker()}
              </p>
              <p className="title">
                {m.initial_setup_auto_adoption_summary_community_title()}
              </p>
            </div>
            <Button
              variant="outlined"
              text={m.initial_setup_auto_adoption_summary_community_button()}
              iconRight="open-in-new-window"
              onClick={() =>
                window.open('https://github.com/DefGuard/defguard/discussions', '_blank')
              }
            />
          </div>
        </div>

        <div className="container">
          <img
            src={ShieldIcon}
            alt={m.initial_setup_auto_adoption_summary_support_icon_alt()}
            className="icon"
          />
          <div className="recommendation-row">
            <div className="kicker-title">
              <p className="kicker">
                {m.initial_setup_auto_adoption_summary_support_kicker()}
              </p>
              <p className="title">
                {m.initial_setup_auto_adoption_summary_support_title()}
              </p>
            </div>
            <Button
              variant="outlined"
              text={m.initial_setup_auto_adoption_summary_support_button()}
              iconRight="open-in-new-window"
              onClick={() =>
                window.open('https://github.com/DefGuard/defguard', '_blank')
              }
            />
          </div>
        </div>
      </div>

      <Controls>
        <div className="right">
          <Button
            text={m.initial_setup_auto_adoption_summary_submit()}
            onClick={handleGoToDefguard}
            loading={isSubmitting}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
