import { useCallback, useEffect, useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { LoadingStep } from '../../../shared/components/LoadingStep/LoadingStep';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { CodeCard } from '../../../shared/defguard-ui/components/CodeCard/CodeCard';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useSSEController } from '../../../shared/hooks/useSSEController';
import { GatewaySetupStep } from '../types';
import { useGatewayWizardStore } from '../useGatewayWizardStore';
import type { SetupEvent, SetupStep, SetupStepId } from './types';

export const SetupGatewayAdoptionStep = () => {
  const setActiveStep = useGatewayWizardStore((s) => s.setActiveStep);
  const gatewayComponentWizardStore = useGatewayWizardStore((s) => s);
  const gatewayAdoptionState = useGatewayWizardStore((s) => s.gatewayAdoptionState);
  const setGatewayAdoptionState = useGatewayWizardStore((s) => s.setGatewayAdoptionState);
  const resetGatewayAdoptionState = useGatewayWizardStore(
    (s) => s.resetGatewayAdoptionState,
  );

  const handleEvent = useCallback(
    (event: SetupEvent) => {
      setGatewayAdoptionState({
        currentStep: event.step,
        isComplete: event.step === 'Done',
        isProcessing: event.step !== 'Done' && !event.error,
        gatewayVersion: event.version ?? null,
        errorMessage: event.error
          ? event.message || m.edge_setup_adoption_error_default()
          : null,
        gatewayLogs: event.logs && event.logs.length > 0 ? [...event.logs] : [],
      });
    },
    [setGatewayAdoptionState],
  );

  const sse = useSSEController<SetupEvent>(
    `/api/v1/network/${gatewayComponentWizardStore.network_id}/gateways/setup`,
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
    useGatewayWizardStore.getState().resetGatewayAdoptionState();
    setActiveStep(GatewaySetupStep.GatewayComponent);
  };

  const handleNext = () => {
    setActiveStep(GatewaySetupStep.Confirmation);
  };

  const steps: SetupStep[] = useMemo(
    () => [
      {
        id: 'CheckingConfiguration',
        title: m.gateway_setup_adoption_checking_configuration(),
      },
      {
        id: 'CheckingAvailability',
        title: m.gateway_setup_adoption_checking_availability({
          ip_or_domain: gatewayComponentWizardStore.ip_or_domain,
          grpc_port: String(gatewayComponentWizardStore.grpc_port),
        }),
      },
      {
        id: 'CheckingVersion',
        title: gatewayAdoptionState.gatewayVersion
          ? m.gateway_setup_adoption_checking_version_with_value({
              gatewayVersion: gatewayAdoptionState.gatewayVersion,
            })
          : m.gateway_setup_adoption_checking_version(),
      },
      {
        id: 'ObtainingCsr',
        title: m.gateway_setup_adoption_obtaining_csr(),
      },
      {
        id: 'SigningCertificate',
        title: m.gateway_setup_adoption_signing_certificate(),
      },
      {
        id: 'ConfiguringTls',
        title: m.gateway_setup_adoption_configuring_tls(),
      },
    ],
    [gatewayComponentWizardStore, gatewayAdoptionState.gatewayVersion],
  );

  const stepDone = useCallback(
    (stepId: SetupStepId): boolean => {
      const stepIndex = steps.findIndex((step) => step.id === stepId);
      const currentStepIndex = gatewayAdoptionState.currentStep
        ? steps.findIndex((step) => step.id === gatewayAdoptionState.currentStep)
        : -1;
      return stepIndex < currentStepIndex || gatewayAdoptionState.isComplete;
    },
    [gatewayAdoptionState.isComplete, gatewayAdoptionState.currentStep, steps],
  );

  const stepLoading = useCallback(
    (stepId: SetupStepId): boolean => {
      return (
        gatewayAdoptionState.isProcessing && gatewayAdoptionState.currentStep === stepId
      );
    },
    [gatewayAdoptionState.isProcessing, gatewayAdoptionState.currentStep],
  );

  const stepError = useCallback(
    (stepId: SetupStepId): string | null => {
      if (
        gatewayAdoptionState.errorMessage &&
        gatewayAdoptionState.currentStep === stepId
      ) {
        return gatewayAdoptionState.errorMessage;
      }
      return null;
    },
    [gatewayAdoptionState.errorMessage, gatewayAdoptionState.currentStep],
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    resetGatewayAdoptionState();
    sse.start();

    return () => {
      sse.stop();
    };
  }, []);

  return (
    <WizardCard>
      <div>
        {steps.map((step) => (
          <LoadingStep
            key={step.id}
            title={step.title}
            loading={stepLoading(step.id)}
            success={stepDone(step.id)}
            error={!!stepError(step.id)}
            errorMessage={stepError(step.id) || undefined}
          >
            {gatewayAdoptionState.gatewayLogs.length > 0 ? (
              <>
                <CodeCard
                  title={m.gateway_setup_adoption_error_log_title()}
                  value={gatewayAdoptionState.gatewayLogs.join('\n')}
                />
                <SizedBox height={ThemeSpacing.Xl} />
              </>
            ) : null}
            <Controls>
              <div className="left">
                <Button
                  variant="primary"
                  text={m.gateway_setup_adoption_controls_retry()}
                  onClick={() => {
                    resetGatewayAdoptionState();
                    sse.restart();
                  }}
                  disabled={gatewayAdoptionState.isProcessing}
                />
              </div>
            </Controls>
          </LoadingStep>
        ))}
      </div>
      <ModalControls
        cancelProps={{
          text: m.gateway_setup_adoption_controls_back(),
          onClick: handleBack,
          disabled: gatewayAdoptionState.isProcessing || gatewayAdoptionState.isComplete,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.gateway_setup_adoption_controls_continue(),
          onClick: handleNext,
          disabled: !gatewayAdoptionState.isComplete || gatewayAdoptionState.isProcessing,
        }}
      />
    </WizardCard>
  );
};
