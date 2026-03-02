import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import api from '../../../../shared/api/api';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
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
        const response = await api.settings.getSettingsEssentials();

        if (isPresent(response.data) && response.data.initial_setup_completed) {
          return;
        }
      } catch (_error) {
        // Ignore errors while API restarts.
      }

      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }

    throw new Error('Timed out waiting for settings essentials.');
  };

  const { mutateAsync: finishSetup } = useMutation({
    mutationKey: ['finish-setup'],
    mutationFn: api.initial_setup.finishSetup,
    meta: {
      invalidate: ['settings_essentials'],
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
      console.error('Failed to finish setup flow:', error);
      Snackbar.error('Failed to finish setup.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <WizardCard className="auto-adoption-summary-step">
      <p className="thank-you">Thank you for choosing Defguard.</p>
      <Divider spacing={ThemeSpacing.Xl} />
      <p className="note">
        Please note that if the host running Defguard is not publicly accessible (i.e., it
        does not have the VPN public IP assigned to it), you must forward the following
        ports to it:
      </p>
      <SizedBox height={ThemeSpacing.Lg} />
      <ul>
        <li>TCP ports 80 and 443</li>
        <li>UDP port {wireguardPort}</li>
      </ul>
      <Divider spacing={ThemeSpacing.Xl2} />
      <p className="encourage">We would encourage you to:</p>
      <SizedBox height={ThemeSpacing.Md} />
      <div className="recommendations">
        <div className="container">
          <img src={FileIcon} alt="Documentation Icon" className="icon" />
          <div className="recommendation-row">
            <div className="kicker-title">
              <p className="kicker">Defguard insides</p>
              <p className="title">
                Get familiar with our security concepts and architecture
              </p>
            </div>
            <Button
              variant="outlined"
              text="Learn more"
              iconRight="open-in-new-window"
              onClick={() => window.open('https://docs.defguard.net/', '_blank')}
            />
          </div>
        </div>

        <div className="container">
          <img src={CommunityIcon} alt="Community Icon" className="icon" />
          <div className="recommendation-row">
            <div className="kicker-title">
              <p className="kicker">Join our community</p>
              <p className="title">Join our community and participate in discussion</p>
            </div>
            <Button
              variant="outlined"
              text="Join now"
              iconRight="open-in-new-window"
              onClick={() =>
                window.open('https://github.com/DefGuard/defguard/discussions', '_blank')
              }
            />
          </div>
        </div>

        <div className="container">
          <img src={ShieldIcon} alt="Security Icon" className="icon" />
          <div className="recommendation-row">
            <div className="kicker-title">
              <p className="kicker">Support Us</p>
              <p className="title">Star us on GitHub</p>
            </div>
            <Button
              variant="outlined"
              text="Go to GitHub"
              iconRight="open-in-new-window"
              onClick={() =>
                window.open('https://github.com/DefGuard/defguard', '_blank')
              }
            />
          </div>
        </div>
      </div>

      <ModalControls
        submitProps={{
          text: 'Go to Defguard',
          onClick: handleGoToDefguard,
          loading: isSubmitting,
        }}
      />
    </WizardCard>
  );
};
