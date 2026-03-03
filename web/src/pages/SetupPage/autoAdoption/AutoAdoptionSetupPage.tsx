import { useQuery } from '@tanstack/react-query';
import { type ReactNode, useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { SetupAutoAdoptionResponse } from '../../../shared/api/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import type { WizardPageStep } from '../../../shared/components/wizard/types';
import { WizardPage } from '../../../shared/components/wizard/WizardPage/WizardPage';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { CodeCard } from '../../../shared/defguard-ui/components/CodeCard/CodeCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useClipboard } from '../../../shared/defguard-ui/hooks/useClipboard';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { downloadText } from '../../../shared/utils/download';
import worldMap from '../assets/world-map.png';
import { AutoAdoptionAdminUserStep } from './steps/AutoAdoptionAdminUserStep';
import { AutoAdoptionMfaSetupStep } from './steps/AutoAdoptionMfaSetupStep';
import { AutoAdoptionSummaryStep } from './steps/AutoAdoptionSummaryStep';
import { AutoAdoptionUrlSettingsStep } from './steps/AutoAdoptionUrlSettingsStep';
import { AutoAdoptionVpnSettingsStep } from './steps/AutoAdoptionVpnSettingsStep';
import { AutoAdoptionSetupStep, type AutoAdoptionSetupStepValue } from './types';
import { useAutoAdoptionSetupWizardStore } from './useAutoAdoptionSetupWizardStore';
import './style.scss';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';

const componentLabel = (component: string): string => {
  if (component === 'edge') return m.initial_setup_auto_adoption_component_edge();
  if (component === 'gateway') return m.initial_setup_auto_adoption_component_gateway();
  return component;
};

const componentLogPrefix = (component: string): string => `[${component.toUpperCase()}]`;

const formatComponentLogs = (component: string, logs: string[]): string =>
  logs
    .flatMap((line) => line.split(/\r?\n/))
    .filter((line) => line.length > 0)
    .map((line) => `${componentLogPrefix(component)} ${line}`)
    .join('\n');

type AutoAdoptionWelcomeContentProps = {
  results: SetupAutoAdoptionResponse['adoption_result'];
};

const AutoAdoptionFailedWelcomeContent = ({
  results,
}: AutoAdoptionWelcomeContentProps) => {
  const { writeToClipboard } = useClipboard();

  return (
    <div className="auto-adoption-welcome-content">
      <Divider spacing={ThemeSpacing.Xl} />
      <div className="summary">
        <p className="summary-title">{m.initial_setup_auto_adoption_failed_summary_title()}</p>
        <SizedBox height={ThemeSpacing.Md} />
        <ul className="status-list">
          <li key={'ca'} className={'status-item success'}>
            <div className="status-row">
              <Icon icon={'check-circle'} />
              <span>{m.initial_setup_auto_adoption_failed_ca_success()}</span>
            </div>
          </li>
          {Object.entries(results).map(([component, state]) => {
            const success = state.success === true;
            const componentLogs = formatComponentLogs(component, state.logs);
            const showErrorLog = !success && componentLogs.length > 0;
            return (
              <li
                key={component}
                className={`status-item ${success ? 'success' : 'error'}`}
              >
                <div className="status-row">
                  <Icon icon={success ? 'check-circle' : 'warning-filled'} />
                  <span>
                    {success
                      ? m.initial_setup_auto_adoption_failed_component_success({
                          component: componentLabel(component),
                        })
                      : m.initial_setup_auto_adoption_failed_component_unsuccessful({
                          component: componentLabel(component),
                        })}
                  </span>
                </div>
                {showErrorLog && (
                  <div className="component-error-log">
                    <CodeCard
                      title={m.initial_setup_auto_adoption_failed_component_error_log_title({
                        component: componentLabel(component),
                      })}
                      value={componentLogs}
                      onCopy={() => {
                        void writeToClipboard(componentLogs);
                      }}
                      onDownload={() => {
                        downloadText(
                          componentLogs,
                          `auto-adoption-error-log-${component}`,
                          'txt',
                        );
                      }}
                    />
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      </div>
      <SizedBox height={ThemeSpacing.Lg} />
      <div className="support-links">
        <div className="support-row">
          <Icon icon="support" />
          <p>
            {m.initial_setup_auto_adoption_failed_support_business_prefix()}{' '}
            <a href="mailto:support@defguard.net">
              {m.initial_setup_auto_adoption_failed_support_business_link()}
            </a>{' '}
            {m.initial_setup_auto_adoption_failed_support_business_suffix()}
          </p>
        </div>
        <div className="support-row">
          <Icon icon="config" />
          <p>
            {m.initial_setup_auto_adoption_failed_support_community_prefix()}{' '}
            <a
              href="https://github.com/DefGuard/defguard/discussions"
              target="_blank"
              rel="noreferrer"
            >
              {m.initial_setup_auto_adoption_failed_support_community_link()}
            </a>
          </p>
        </div>
      </div>
    </div>
  );
};

type AutoAdoptionSuccessWelcomeContentProps = {
  onStartFlow: () => void;
};

const AutoAdoptionSuccessWelcomeContent = ({
  onStartFlow,
}: AutoAdoptionSuccessWelcomeContentProps) => (
  <div className="auto-adoption-welcome-content">
    <Divider spacing={ThemeSpacing.Lg} />
    <p>{m.initial_setup_auto_adoption_success_guide_intro()}</p>
    <br />
    <p>
      {m.initial_setup_auto_adoption_success_guide_description()}
    </p>
    <SizedBox height={ThemeSpacing.Xl} />
    <Controls>
      <Button
        text={m.initial_setup_auto_adoption_success_start_button()}
        onClick={() => {
          onStartFlow();
        }}
      />
    </Controls>
  </div>
);

export const AutoAdoptionSetupPage = () => {
  const activeStep = useAutoAdoptionSetupWizardStore((s) => s.activeStep);
  const isAutoAdoptionFlowStarted = useAutoAdoptionSetupWizardStore(
    (s) => s.isAutoAdoptionFlowStarted,
  );
  const startFlow = useAutoAdoptionSetupWizardStore((s) => s.startFlow);

  const { data: statusData } = useQuery({
    queryKey: ['initial_setup', 'auto_adoption', 'status'],
    queryFn: api.initial_setup.getAutoAdoptionResult,
    select: (response) => response.data,
    refetchInterval: 3000,
  });

  const results = statusData?.adoption_result;

  const hasFailedResult = Object.values((isPresent(results) && results) ?? {}).some(
    (result) => result.success === false,
  );

  const stepsConfig = useMemo(
    (): Record<AutoAdoptionSetupStepValue, WizardPageStep> => ({
      adminUser: {
        id: AutoAdoptionSetupStep.AdminUser,
        order: 1,
        label: m.initial_setup_auto_adoption_step_admin_user_label(),
        description: m.initial_setup_auto_adoption_step_admin_user_description(),
      },
      urlSettings: {
        id: AutoAdoptionSetupStep.UrlSettings,
        order: 2,
        label: m.initial_setup_auto_adoption_step_url_settings_label(),
        description: m.initial_setup_auto_adoption_step_url_settings_description(),
      },
      vpnSettings: {
        id: AutoAdoptionSetupStep.VpnSettings,
        order: 3,
        label: m.initial_setup_auto_adoption_step_vpn_settings_label(),
        description: m.initial_setup_auto_adoption_step_vpn_settings_description(),
      },
      mfaSetup: {
        id: AutoAdoptionSetupStep.MfaSetup,
        order: 4,
        label: m.initial_setup_auto_adoption_step_mfa_setup_label(),
        description: m.initial_setup_auto_adoption_step_mfa_setup_description(),
      },
      summary: {
        id: AutoAdoptionSetupStep.Summary,
        order: 5,
        label: m.initial_setup_auto_adoption_step_summary_label(),
        description: m.initial_setup_auto_adoption_step_summary_description(),
      },
    }),
    [],
  );

  const stepsComponents = useMemo(
    (): Record<AutoAdoptionSetupStepValue, ReactNode> => ({
      adminUser: <AutoAdoptionAdminUserStep />,
      urlSettings: <AutoAdoptionUrlSettingsStep />,
      vpnSettings: <AutoAdoptionVpnSettingsStep />,
      mfaSetup: <AutoAdoptionMfaSetupStep />,
      summary: <AutoAdoptionSummaryStep />,
    }),
    [],
  );

  const subtitle = hasFailedResult
    ? m.initial_setup_auto_adoption_welcome_subtitle_failed()
    : m.initial_setup_auto_adoption_welcome_subtitle_success();

  if (!results) {
    return null;
  }

  return (
    <WizardPage
      id="auto-adoption-setup-wizard"
      activeStep={activeStep}
      subtitle={m.initial_setup_auto_adoption_wizard_subtitle()}
      title={m.initial_setup_auto_adoption_wizard_title()}
      steps={stepsConfig}
      isOnWelcomePage={!isAutoAdoptionFlowStarted}
      welcomePageConfig={{
        title: m.initial_setup_auto_adoption_welcome_title(),
        subtitle,
        content: hasFailedResult ? (
          <AutoAdoptionFailedWelcomeContent results={results} />
        ) : (
          <AutoAdoptionSuccessWelcomeContent onStartFlow={startFlow} />
        ),
        media: <img src={worldMap} alt="World map" />,
        displayDocs: false,
      }}
    >
      {stepsComponents[activeStep]}
    </WizardPage>
  );
};
