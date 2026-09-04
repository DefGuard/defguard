import type { MfaFlowAssignment } from '../../../../shared/api/types';

export type SplitAssignments = {
  defaultAssignment: MfaFlowAssignment | undefined;
  overrides: MfaFlowAssignment[];
};

export const splitAssignments = (assignments: MfaFlowAssignment[]): SplitAssignments => ({
  defaultAssignment: assignments.find((assignment) => assignment.is_default),
  overrides: assignments.filter((assignment) => !assignment.is_default),
});

/** Include group IDs so overrides using the same flow get distinct keys. */
export const overrideKey = (override: MfaFlowAssignment) =>
  `${override.flow_id}:${override.group_ids.join(',')}`;

/** Returns unassigned flows and keeps the current flow selectable. */
export const selectableFlows = <T extends { id: number }>(
  flows: T[],
  assignments: MfaFlowAssignment[],
  currentFlowId: number | undefined,
): T[] =>
  flows.filter(
    (flow) =>
      flow.id === currentFlowId ||
      !assignments.some((assignment) => assignment.flow_id === flow.id),
  );

// Keep the fallback last because the server matches assignments in order.
const withDefaultLast = (
  overrides: MfaFlowAssignment[],
  defaultAssignment: MfaFlowAssignment | undefined,
): MfaFlowAssignment[] =>
  defaultAssignment === undefined ? overrides : [...overrides, defaultAssignment];

export const setDefaultFlow = (
  assignments: MfaFlowAssignment[],
  flowId: number,
): MfaFlowAssignment[] => {
  const { overrides } = splitAssignments(assignments);
  return withDefaultLast(overrides, { flow_id: flowId, is_default: true, group_ids: [] });
};

/** Replaces an override at `index` or appends one when omitted; `groupIds` must be non-empty. */
export const upsertOverride = (
  assignments: MfaFlowAssignment[],
  index: number | undefined,
  flowId: number,
  groupIds: number[],
): MfaFlowAssignment[] => {
  const { defaultAssignment, overrides } = splitAssignments(assignments);
  const override: MfaFlowAssignment = {
    flow_id: flowId,
    is_default: false,
    group_ids: groupIds,
  };
  const next =
    index === undefined
      ? [...overrides, override]
      : overrides.map((current, at) => (at === index ? override : current));
  return withDefaultLast(next, defaultAssignment);
};

/** Applies the override order the server uses for matching. */
export const reorderOverrides = (
  assignments: MfaFlowAssignment[],
  overrides: MfaFlowAssignment[],
): MfaFlowAssignment[] =>
  withDefaultLast(overrides, splitAssignments(assignments).defaultAssignment);

export const removeOverride = (
  assignments: MfaFlowAssignment[],
  index: number,
): MfaFlowAssignment[] => {
  const { defaultAssignment, overrides } = splitAssignments(assignments);
  return withDefaultLast(
    overrides.filter((_, at) => at !== index),
    defaultAssignment,
  );
};
