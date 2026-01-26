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
import { EdgeSetupStep } from '../types';
import { useEdgeWizardStore } from '../useEdgeWizardStore';
import type { SetupEvent, SetupStep, SetupStepId } from './types';
import { useSSEController } from './useSSEController';

export const SetupEdgeAdaptationStep = () => {
  const setActiveStep = useEdgeWizardStore((s) => s.setActiveStep);
  const edgeComponentWizardStore = useEdgeWizardStore((s) => s);
  const edgeAdaptationState = useEdgeWizardStore((s) => s.edgeAdaptationState);
  const setEdgeAdaptationState = useEdgeWizardStore((s) => s.setEdgeAdaptationState);
  const resetEdgeAdaptationState = useEdgeWizardStore((s) => s.resetEdgeAdaptationState);

  const handleEvent = useCallback(
    (event: SetupEvent) => {
      setEdgeAdaptationState({
        currentStep: event.step,
        isComplete: event.step === 'Done',
        isProcessing: event.step !== 'Done' && !event.error,
        proxyVersion: event.proxy_version ?? null,
        errorMessage: event.error
          ? event.message || m.edge_setup_adaptation_error_default()
          : null,
        proxyLogs: event.logs && event.logs.length > 0 ? [...event.logs] : [],
      });
    },
    [setEdgeAdaptationState],
  );

  const sse = useSSEController<SetupEvent>(
    '/api/v1/proxy/setup/stream',
    {
      ip_or_domain: edgeComponentWizardStore.ip_or_domain,
      grpc_port: edgeComponentWizardStore.grpc_port,
      common_name: edgeComponentWizardStore.common_name,
    },
    {
      onOpen: () =>
        setEdgeAdaptationState({
          ...edgeAdaptationState,
          isProcessing: true,
        }),
      onMessage: handleEvent,
      onError: () => {
        setEdgeAdaptationState({
          ...edgeAdaptationState,
          isProcessing: false,
        });
      },
    },
  );

  const handleBack = () => {
    useEdgeWizardStore.getState().resetEdgeAdaptationState();
    setActiveStep(EdgeSetupStep.EdgeComponent);
  };

  const handleNext = () => {
    setActiveStep(EdgeSetupStep.Confirmation);
  };

  const steps: SetupStep[] = useMemo(
    () => [
      {
        id: 'CheckingConfiguration',
        title: m.edge_setup_adaptation_checking_configuration(),
      },
      {
        id: 'CheckingAvailability',
        title: m.edge_setup_adaptation_checking_availability({
          ip_or_domain: edgeComponentWizardStore.ip_or_domain,
          grpc_port: edgeComponentWizardStore.grpc_port.toString(),
        }),
      },
      {
        id: 'CheckingVersion',
        title: edgeAdaptationState.proxyVersion
          ? m.edge_setup_adaptation_checking_version_with_value({
              proxyVersion: edgeAdaptationState.proxyVersion,
            })
          : m.edge_setup_adaptation_checking_version(),
      },
      {
        id: 'ObtainingCsr',
        title: m.edge_setup_adaptation_obtaining_csr(),
      },
      {
        id: 'SigningCertificate',
        title: m.edge_setup_adaptation_signing_certificate(),
      },
      {
        id: 'ConfiguringTls',
        title: m.edge_setup_adaptation_configuring_tls(),
      },
    ],
    [edgeComponentWizardStore, edgeAdaptationState.proxyVersion],
  );

  const stepDone = useCallback(
    (stepId: SetupStepId): boolean => {
      const stepIndex = steps.findIndex((step) => step.id === stepId);
      const currentStepIndex = edgeAdaptationState.currentStep
        ? steps.findIndex((step) => step.id === edgeAdaptationState.currentStep)
        : -1;
      return stepIndex < currentStepIndex || edgeAdaptationState.isComplete;
    },
    [edgeAdaptationState.isComplete, edgeAdaptationState.currentStep, steps],
  );

  const stepLoading = useCallback(
    (stepId: SetupStepId): boolean => {
      return (
        edgeAdaptationState.isProcessing && edgeAdaptationState.currentStep === stepId
      );
    },
    [edgeAdaptationState.isProcessing, edgeAdaptationState.currentStep],
  );

  const stepError = useCallback(
    (stepId: SetupStepId): string | null => {
      if (
        edgeAdaptationState.errorMessage &&
        edgeAdaptationState.currentStep === stepId
      ) {
        return edgeAdaptationState.errorMessage;
      }
      return null;
    },
    [edgeAdaptationState.errorMessage, edgeAdaptationState.currentStep],
  );

  useEffect(() => {
    resetEdgeAdaptationState();
    sse.start();

    return () => {
      sse.stop();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
            {edgeAdaptationState.proxyLogs.length > 0 ? (
              <>
                <CodeCard
                  title={m.edge_setup_adaptation_error_log_title()}
                  value={edgeAdaptationState.proxyLogs.join('\n')}
                />
                <SizedBox height={ThemeSpacing.Xl} />
              </>
            ) : null}
            <Controls>
              <div className="left">
                <Button
                  variant="primary"
                  text={m.edge_setup_adaptation_controls_retry()}
                  onClick={() => {
                    resetEdgeAdaptationState();
                    sse.restart();
                  }}
                  disabled={edgeAdaptationState.isProcessing}
                />
              </div>
            </Controls>
          </LoadingStep>
        ))}
      </div>
      <ModalControls
        cancelProps={{
          text: m.edge_setup_adaptation_controls_back(),
          onClick: handleBack,
          disabled: edgeAdaptationState.isProcessing || edgeAdaptationState.isComplete,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.edge_setup_adaptation_controls_continue(),
          onClick: handleNext,
          disabled: !edgeAdaptationState.isComplete || edgeAdaptationState.isProcessing,
        }}
      />
    </WizardCard>
  );
};
