import type { useNavigate } from '@tanstack/react-router';
import { useAddPostureCheckWizardStore } from './useAddPostureCheckWizardStore';

type NavigateFn = ReturnType<typeof useNavigate>;

export const closeAddPostureCheckWizard = (navigate: NavigateFn) => {
  void navigate({ to: '/acl/posture-checks', replace: true }).then(() => {
    setTimeout(() => {
      useAddPostureCheckWizardStore.getState().reset();
    }, 100);
  });
};
