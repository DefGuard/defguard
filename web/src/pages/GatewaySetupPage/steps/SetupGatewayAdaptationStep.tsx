import { useCallback, useEffect, useMemo } from 'react';
import { useSSEController } from '../../../hooks/useSSEController';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { LoadingStep } from '../../../shared/components/LoadingStep/LoadingStep';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { CodeCard } from '../../../shared/defguard-ui/components/CodeCard/CodeCard';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { GatewaySetupStep } from '../types';
import { useGatewayWizardStore } from '../useGatewayWizardStore';
import type { SetupEvent, SetupStep, SetupStepId } from './types';

export const SetupGatewayAdaptationStep = () => {
  const setActiveStep = useGatewayWizardStore((s) => s.setActiveStep);
  const gatewayComponentWizardStore = useGatewayWizardStore((s) => s);
  const gatewayAdaptationState = useGatewayWizardStore((s) => s.gatewayAdaptationState);
  const setGatewayAdaptationState = useGatewayWizardStore(
    (s) => s.setGatewayAdaptationState,
  );
  const resetGatewayAdaptationState = useGatewayWizardStore(
    (s) => s.resetGatewayAdaptationState,
  );

  // take networkid from params of wizard start

  const handleEvent = useCallback(
    (event: SetupEvent) => {
      setGatewayAdaptationState({
        currentStep: event.step,
        isComplete: event.step === 'Done',
        isProcessing: event.step !== 'Done' && !event.error,
        gatewayVersion: event.version ?? null,
        errorMessage: event.error
          ? event.message || m.edge_setup_adaptation_error_default()
          : null,
        gatewayLogs: event.logs && event.logs.length > 0 ? [...event.logs] : [],
      });
    },
    [setGatewayAdaptationState],
  );

  const sse = useSSEController<SetupEvent>(
    '/api/v1/gateway/setup/stream',
    {
      ip_or_domain: gatewayComponentWizardStore.ip_or_domain,
      grpc_port: gatewayComponentWizardStore.grpc_port,
      common_name: gatewayComponentWizardStore.common_name,
      network_id: gatewayComponentWizardStore.network_id,
    },
    {
      onMessage: handleEvent,
    },
  );

  const handleBack = () => {
    useGatewayWizardStore.getState().resetGatewayAdaptationState();
    setActiveStep(GatewaySetupStep.GatewayComponent);
  };

  const handleNext = () => {
    setActiveStep(GatewaySetupStep.Confirmation);
  };

  const steps: SetupStep[] = useMemo(
    () => [
      {
        id: 'CheckingConfiguration',
        title: m.gateway_setup_adaptation_checking_configuration(),
      },
      {
        id: 'CheckingAvailability',
        title: `Checking if Gateway is available at: ${gatewayComponentWizardStore.ip_or_domain}:${gatewayComponentWizardStore.grpc_port}`,
      },
      {
        id: 'CheckingVersion',
        title: gatewayAdaptationState.gatewayVersion
          ? `Checking Gateway version: ${gatewayAdaptationState.gatewayVersion}`
          : m.gateway_setup_adaptation_checking_version(),
      },
      {
        id: 'ObtainingCsr',
        title: m.gateway_setup_adaptation_obtaining_csr(),
      },
      {
        id: 'SigningCertificate',
        title: m.gateway_setup_adaptation_signing_certificate(),
      },
      {
        id: 'ConfiguringTls',
        title: m.gateway_setup_adaptation_configuring_tls(),
      },
    ],
    [gatewayComponentWizardStore, gatewayAdaptationState.gatewayVersion],
  );

  const stepDone = useCallback(
    (stepId: SetupStepId): boolean => {
      const stepIndex = steps.findIndex((step) => step.id === stepId);
      const currentStepIndex = gatewayAdaptationState.currentStep
        ? steps.findIndex((step) => step.id === gatewayAdaptationState.currentStep)
        : -1;
      return stepIndex < currentStepIndex || gatewayAdaptationState.isComplete;
    },
    [gatewayAdaptationState.isComplete, gatewayAdaptationState.currentStep, steps],
  );

  const stepLoading = useCallback(
    (stepId: SetupStepId): boolean => {
      return (
        gatewayAdaptationState.isProcessing &&
        gatewayAdaptationState.currentStep === stepId
      );
    },
    [gatewayAdaptationState.isProcessing, gatewayAdaptationState.currentStep],
  );

  const stepError = useCallback(
    (stepId: SetupStepId): string | null => {
      if (
        gatewayAdaptationState.errorMessage &&
        gatewayAdaptationState.currentStep === stepId
      ) {
        return gatewayAdaptationState.errorMessage;
      }
      return null;
    },
    [gatewayAdaptationState.errorMessage, gatewayAdaptationState.currentStep],
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    resetGatewayAdaptationState();
    sse.start();

    return () => {
      sse.stop();
    };
  }, []);

  return (
    <WizardCard>
      <div>
        {steps.map((step, index) => (
          <LoadingStep
            key={index}
            title={step.title}
            loading={stepLoading(step.id)}
            success={stepDone(step.id)}
            error={!!stepError(step.id)}
            errorMessage={stepError(step.id) || undefined}
          >
            {gatewayAdaptationState.gatewayLogs.length > 0 ? (
              <>
                <CodeCard
                  title={m.gateway_setup_adaptation_error_log_title()}
                  value={gatewayAdaptationState.gatewayLogs.join('\n')}
                />
                <SizedBox height={ThemeSpacing.Xl} />
              </>
            ) : null}
            <Controls>
              <div className="left">
                <Button
                  variant="primary"
                  text={m.gateway_setup_adaptation_controls_retry()}
                  onClick={() => {
                    resetGatewayAdaptationState();
                    sse.restart();
                  }}
                  disabled={gatewayAdaptationState.isProcessing}
                />
              </div>
            </Controls>
          </LoadingStep>
        ))}
      </div>
      <ModalControls
        cancelProps={{
          text: m.gateway_setup_adaptation_controls_back(),
          onClick: handleBack,
          disabled:
            gatewayAdaptationState.isProcessing || gatewayAdaptationState.isComplete,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.gateway_setup_adaptation_controls_continue(),
          onClick: handleNext,
          disabled:
            !gatewayAdaptationState.isComplete || gatewayAdaptationState.isProcessing,
        }}
      />
    </WizardCard>
  );
};
