export type PostureChecksSectionState = {
  hasAnyPostureChecks: boolean;
  hasAssignedPostureChecks: boolean;
  locked: boolean;
  showAssignButton: boolean;
  showLockedButton: boolean;
  showAssignedPostureChecks: boolean;
  showEmptyState: boolean;
};

type Input = {
  assignedPostureChecksCount: number;
  canUseEnterprise: boolean | undefined;
  postureChecksCount: number;
};

// Keep the edit-location posture-check section aligned with the Figma states.
export const getPostureChecksSectionState = ({
  assignedPostureChecksCount,
  canUseEnterprise,
  postureChecksCount,
}: Input): PostureChecksSectionState => {
  const locked = canUseEnterprise === false;
  const hasAnyPostureChecks = postureChecksCount > 0;
  const hasAssignedPostureChecks = assignedPostureChecksCount > 0;

  return {
    hasAnyPostureChecks,
    hasAssignedPostureChecks,
    locked,
    showAssignButton: !locked && hasAnyPostureChecks && !hasAssignedPostureChecks,
    showLockedButton: locked,
    showAssignedPostureChecks: !locked && hasAssignedPostureChecks,
    showEmptyState: !locked && !hasAnyPostureChecks,
  };
};
