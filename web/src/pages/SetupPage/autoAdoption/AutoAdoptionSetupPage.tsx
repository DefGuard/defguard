import { useQuery } from '@tanstack/react-query';
import { type ReactNode, useMemo } from 'react';
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
  if (component === 'edge') return 'Edge';
  if (component === 'gateway') return 'Gateway';
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
        <p className="summary-title">Error summary:</p>
        <SizedBox height={ThemeSpacing.Md} />
        <ul className="status-list">
          <li key={'ca'} className={'status-item success'}>
            <div className="status-row">
              <Icon icon={'check-circle'} />
              <span>Certificate Authority setup successful.</span>
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
                    {componentLabel(component)} setup{' '}
                    {success ? 'successful' : 'unsuccessful'}
                  </span>
                </div>
                {showErrorLog && (
                  <div className="component-error-log">
                    <CodeCard
                      title={`${componentLabel(component)} error log`}
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
            If you are a Business or Enterprise customer, please{' '}
            <a href="mailto:support@defguard.net">contact our support team</a> and provide
            the logs you see in the error summary section above.
          </p>
        </div>
        <div className="support-row">
          <Icon icon="config" />
          <p>
            If you are an Open Source or Free plan user, find support on{' '}
            <a
              href="https://github.com/DefGuard/defguard/discussions"
              target="_blank"
              rel="noreferrer"
            >
              Github Discussions.
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
    <p>This guide will walk you through the process.</p>
    <br />
    <p>
      If you would like to understand some basic Defguard concepts, each screen includes
      links to documentation as well as short videos with explanations that you can watch
      directly during the setup process.
    </p>
    <SizedBox height={ThemeSpacing.Xl} />
    <Controls>
      <Button
        text="Start Defguard configuration"
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
        label: 'Create Admin User',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      urlSettings: {
        id: AutoAdoptionSetupStep.UrlSettings,
        order: 2,
        label: 'Internal and external URL settings',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      vpnSettings: {
        id: AutoAdoptionSetupStep.VpnSettings,
        order: 3,
        label: 'VPN Public and Internal Settings',
        description:
          'Manage core details and connection parameters for your VPN location.',
      },
      mfaSetup: {
        id: AutoAdoptionSetupStep.MfaSetup,
        order: 4,
        label: 'Multi-Factor Authentication',
        description: 'You can enable Multi-Factor Authentication (MFA) for your VPN.',
      },
      summary: {
        id: AutoAdoptionSetupStep.Summary,
        order: 5,
        label: 'Summary',
        description: 'Everything is set up and ready to go!',
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
    ? 'Unfortunately, the automated setup for some components did not complete successfully. Find detailed errors below.'
    : 'We have successfully configured all the necessary components (gateway and edge) using Docker for this instance. Now, we need to configure some general settings.';

  if (!results) {
    return null;
  }

  return (
    <WizardPage
      id="auto-adoption-setup-wizard"
      activeStep={activeStep}
      subtitle="Complete the final three steps to fully configure DefGuard."
      title="Defguard configuration"
      steps={stepsConfig}
      isOnWelcomePage={!isAutoAdoptionFlowStarted}
      welcomePageConfig={{
        title: 'Welcome to Defguard.',
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
