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
import type { SetupEvent, SetupStep, SetupStepId } from '../../EdgeSetupPage/steps/types';
import { SetupPageStep } from '../types';
import { useSetupWizardStore } from '../useSetupWizardStore';

export const SetupEdgeAdoptionStep = () => {
  const setActiveStep = useSetupWizardStore((s) => s.setActiveStep);
  const setupWizardStore = useSetupWizardStore((s) => s);
  const edgeAdoptionState = useSetupWizardStore((s) => s.edgeAdoptionState);
  const setEdgeAdoptionState = useSetupWizardStore((s) => s.setEdgeAdoptionState);
  const resetEdgeAdoptionState = useSetupWizardStore((s) => s.resetEdgeAdoptionState);

  const handleEvent = useCallback(
    (event: SetupEvent) => {
      setEdgeAdoptionState({
        currentStep: event.step,
        isComplete: event.step === 'Done',
        isProcessing: event.step !== 'Done' && !event.error,
        proxyVersion: event.version ?? null,
        errorMessage: event.error
          ? event.message || m.edge_setup_adoption_error_default()
          : null,
        proxyLogs: event.logs && event.logs.length > 0 ? [...event.logs] : [],
      });
    },
    [setEdgeAdoptionState],
  );

  const sse = useSSEController<SetupEvent>(
    '/api/v1/proxy/setup/stream',
    {
      ip_or_domain: setupWizardStore.ip_or_domain,
      grpc_port: setupWizardStore.grpc_port,
      common_name: setupWizardStore.common_name,
    },
    {
      onMessage: handleEvent,
    },
  );

  const handleBack = () => {
    useSetupWizardStore.getState().resetEdgeAdoptionState();
    setActiveStep(SetupPageStep.EdgeComponent);
  };

  const handleNext = async () => {
    setActiveStep(SetupPageStep.Confirmation);
  };

  const steps: SetupStep[] = useMemo(
    () => [
      {
        id: 'CheckingConfiguration',
        title: m.edge_setup_adoption_checking_configuration(),
      },
      {
        id: 'CheckingAvailability',
        title: m.edge_setup_adoption_checking_availability({
          ip_or_domain: setupWizardStore.ip_or_domain,
          grpc_port: setupWizardStore.grpc_port.toString(),
        }),
      },
      {
        id: 'CheckingVersion',
        title: edgeAdoptionState.proxyVersion
          ? m.edge_setup_adoption_checking_version_with_value({
              proxyVersion: edgeAdoptionState.proxyVersion,
            })
          : m.edge_setup_adoption_checking_version(),
      },
      {
        id: 'ObtainingCsr',
        title: m.edge_setup_adoption_obtaining_csr(),
      },
      {
        id: 'SigningCertificate',
        title: m.edge_setup_adoption_signing_certificate(),
      },
      {
        id: 'ConfiguringTls',
        title: m.edge_setup_adoption_configuring_tls(),
      },
    ],
    [setupWizardStore, edgeAdoptionState.proxyVersion],
  );

  const stepDone = useCallback(
    (stepId: SetupStepId): boolean => {
      const stepIndex = steps.findIndex((step) => step.id === stepId);
      const currentStepIndex = edgeAdoptionState.currentStep
        ? steps.findIndex((step) => step.id === edgeAdoptionState.currentStep)
        : -1;
      return stepIndex < currentStepIndex || edgeAdoptionState.isComplete;
    },
    [edgeAdoptionState.isComplete, edgeAdoptionState.currentStep, steps],
  );

  const stepLoading = useCallback(
    (stepId: SetupStepId): boolean => {
      return edgeAdoptionState.isProcessing && edgeAdoptionState.currentStep === stepId;
    },
    [edgeAdoptionState.isProcessing, edgeAdoptionState.currentStep],
  );

  const stepError = useCallback(
    (stepId: SetupStepId): string | null => {
      if (edgeAdoptionState.errorMessage && edgeAdoptionState.currentStep === stepId) {
        return edgeAdoptionState.errorMessage;
      }
      return null;
    },
    [edgeAdoptionState.errorMessage, edgeAdoptionState.currentStep],
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    resetEdgeAdoptionState();
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
            {edgeAdoptionState.proxyLogs.length > 0 ? (
              <>
                <CodeCard
                  title={m.edge_setup_adoption_error_log_title()}
                  value={edgeAdoptionState.proxyLogs.join('\n')}
                />
                <SizedBox height={ThemeSpacing.Xl} />
              </>
            ) : null}
            <Controls>
              <div className="left">
                <Button
                  variant="primary"
                  text={m.edge_setup_adoption_controls_retry()}
                  onClick={() => {
                    resetEdgeAdoptionState();
                    sse.restart();
                  }}
                  disabled={edgeAdoptionState.isProcessing}
                />
              </div>
            </Controls>
          </LoadingStep>
        ))}
      </div>
      <ModalControls
        cancelProps={{
          text: m.edge_setup_adoption_controls_back(),
          onClick: handleBack,
          disabled: edgeAdoptionState.isProcessing || edgeAdoptionState.isComplete,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.edge_setup_adoption_controls_continue(),
          onClick: handleNext,
          disabled: !edgeAdoptionState.isComplete || edgeAdoptionState.isProcessing,
        }}
      />
    </WizardCard>
  );
};
