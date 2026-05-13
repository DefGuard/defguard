import { useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useCallback, useEffect, useMemo } from 'react';
import { m } from '../../paraglide/messages';
import type { WizardPageStep } from '../../shared/components/wizard/types';
import { WizardPage } from '../../shared/components/wizard/WizardPage/WizardPage';
import { getDevicePostureVersionMetadataQueryOptions } from '../../shared/query';
import { closeAddPostureCheckWizard } from './navigation';
import './style.scss';
import { getPostureCheckVersionValues } from '../PostureChecksPage/types';
import { AddPostureCheckClientVersionStep } from './steps/AddPostureCheckClientVersionStep';
import { AddPostureCheckDetailsStep } from './steps/AddPostureCheckDetailsStep';
import { AddPostureCheckOperatingSystemsStep } from './steps/AddPostureCheckOperatingSystemsStep';
import { AddPostureCheckSummaryStep } from './steps/AddPostureCheckSummaryStep';
import { AddPostureCheckWizardStep, type AddPostureCheckWizardStepValue } from './types';
import { useAddPostureCheckWizardStore } from './useAddPostureCheckWizardStore';

export const AddPostureCheckWizardPage = () => {
  const activeStep = useAddPostureCheckWizardStore((s) => s.activeStep);
  const syncVersionValues = useAddPostureCheckWizardStore((s) => s.syncVersionValues);
  const navigate = useNavigate();
  const { data: versionMetadata } = useSuspenseQuery(
    getDevicePostureVersionMetadataQueryOptions,
  );
  const versionValues = useMemo(
    () => getPostureCheckVersionValues(versionMetadata),
    [versionMetadata],
  );

  useEffect(() => {
    syncVersionValues(versionValues);
  }, [syncVersionValues, versionValues]);

  const onClose = useCallback(() => {
    closeAddPostureCheckWizard(navigate);
  }, [navigate]);

  const steps = useMemo(
    (): Record<AddPostureCheckWizardStepValue, ReactNode> => ({
      [AddPostureCheckWizardStep.OperatingSystems]: (
        <AddPostureCheckOperatingSystemsStep versionValues={versionValues} />
      ),
      [AddPostureCheckWizardStep.ClientVersion]: (
        <AddPostureCheckClientVersionStep versionValues={versionValues} />
      ),
      [AddPostureCheckWizardStep.Details]: <AddPostureCheckDetailsStep />,
      [AddPostureCheckWizardStep.Summary]: <AddPostureCheckSummaryStep />,
    }),
    [versionValues],
  );

  const stepsConfig = useMemo(
    (): Record<AddPostureCheckWizardStepValue, WizardPageStep> => ({
      [AddPostureCheckWizardStep.OperatingSystems]: {
        id: AddPostureCheckWizardStep.OperatingSystems,
        label: m.posture_checks_wizard_step_operating_systems(),
        order: 0,
        description: m.posture_checks_wizard_step_operating_systems_description(),
      },
      [AddPostureCheckWizardStep.ClientVersion]: {
        id: AddPostureCheckWizardStep.ClientVersion,
        label: m.posture_checks_wizard_step_client_version(),
        order: 1,
        description: m.posture_checks_wizard_step_client_version_description(),
      },
      [AddPostureCheckWizardStep.Details]: {
        id: AddPostureCheckWizardStep.Details,
        label: m.posture_checks_wizard_step_details(),
        order: 2,
        description: m.posture_checks_wizard_step_details_description(),
      },
      [AddPostureCheckWizardStep.Summary]: {
        id: AddPostureCheckWizardStep.Summary,
        label: m.posture_checks_wizard_step_summary(),
        order: 3,
        description: m.posture_checks_wizard_step_summary_description(),
      },
    }),
    [],
  );

  return (
    <WizardPage
      activeStep={activeStep}
      steps={stepsConfig}
      title={m.posture_checks_wizard_title()}
      subtitle={m.posture_checks_wizard_subtitle()}
      onClose={onClose}
    >
      {steps[activeStep]}
    </WizardPage>
  );
};
