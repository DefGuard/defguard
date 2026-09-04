import { describe, expect, it } from 'vitest';
import {
  overrideKey,
  removeOverride,
  reorderOverrides,
  selectableFlows,
  setDefaultFlow,
  splitAssignments,
  upsertOverride,
} from '../src/pages/EditLocationPage/components/LocationMfaSection/assignments';
import type { MfaFlowAssignment } from '../src/shared/api/types';

const defaultFlow: MfaFlowAssignment = {
  flow_id: 1,
  is_default: true,
  group_ids: [],
};

const firstOverride: MfaFlowAssignment = {
  flow_id: 2,
  is_default: false,
  group_ids: [10, 11],
};

const secondOverride: MfaFlowAssignment = {
  flow_id: 3,
  is_default: false,
  group_ids: [12],
};

describe('splitAssignments', () => {
  it('separates the default from the overrides', () => {
    const result = splitAssignments([firstOverride, defaultFlow, secondOverride]);

    expect(result.defaultAssignment).toEqual(defaultFlow);
    expect(result.overrides).toEqual([firstOverride, secondOverride]);
  });

  it('reports no default when none is designated', () => {
    expect(splitAssignments([firstOverride]).defaultAssignment).toBeUndefined();
  });
});

describe('setDefaultFlow', () => {
  it('adds the default last when none exists', () => {
    expect(setDefaultFlow([firstOverride], 9)).toEqual([
      firstOverride,
      { flow_id: 9, is_default: true, group_ids: [] },
    ]);
  });

  it('replaces the existing default without touching the overrides', () => {
    expect(setDefaultFlow([firstOverride, defaultFlow], 9)).toEqual([
      firstOverride,
      { flow_id: 9, is_default: true, group_ids: [] },
    ]);
  });

  it('moves a misplaced default back to last position', () => {
    expect(setDefaultFlow([defaultFlow, firstOverride], 1)).toEqual([
      firstOverride,
      defaultFlow,
    ]);
  });

  it('never scopes the default to a group', () => {
    const withGroups: MfaFlowAssignment = {
      flow_id: 1,
      is_default: true,
      group_ids: [10],
    };

    expect(setDefaultFlow([withGroups], 1)).toEqual([defaultFlow]);
  });
});

describe('upsertOverride', () => {
  it('appends a new override before the default', () => {
    expect(upsertOverride([defaultFlow], undefined, 3, [12])).toEqual([
      secondOverride,
      defaultFlow,
    ]);
  });

  it('replaces the override at the given index in place', () => {
    const result = upsertOverride(
      [firstOverride, secondOverride, defaultFlow],
      0,
      7,
      [20],
    );

    expect(result).toEqual([
      { flow_id: 7, is_default: false, group_ids: [20] },
      secondOverride,
      defaultFlow,
    ]);
  });

  it('keeps working when no default is designated yet', () => {
    expect(upsertOverride([], undefined, 3, [12])).toEqual([secondOverride]);
  });
});

describe('reorderOverrides', () => {
  it('applies the new override order and keeps the default last', () => {
    const result = reorderOverrides(
      [firstOverride, secondOverride, defaultFlow],
      [secondOverride, firstOverride],
    );

    expect(result).toEqual([secondOverride, firstOverride, defaultFlow]);
  });

  it('never lets a reorder place an override after the default', () => {
    const result = reorderOverrides([firstOverride, defaultFlow], [firstOverride]);

    expect(result.at(-1)).toEqual(defaultFlow);
  });

  it('works when no default is designated', () => {
    expect(reorderOverrides([firstOverride], [firstOverride])).toEqual([firstOverride]);
  });
});

describe('removeOverride', () => {
  it('drops only the named override', () => {
    expect(removeOverride([firstOverride, secondOverride, defaultFlow], 0)).toEqual([
      secondOverride,
      defaultFlow,
    ]);
  });

  it('leaves the default in place when the last override goes', () => {
    expect(removeOverride([firstOverride, defaultFlow], 0)).toEqual([defaultFlow]);
  });

  it('keeps exactly one default across a sequence of edits', () => {
    let assignments = setDefaultFlow([], 1);
    assignments = upsertOverride(assignments, undefined, 2, [10, 11]);
    assignments = upsertOverride(assignments, undefined, 3, [12]);
    assignments = removeOverride(assignments, 0);
    assignments = setDefaultFlow(assignments, 5);

    expect(assignments.filter((assignment) => assignment.is_default)).toHaveLength(1);
    expect(assignments.at(-1)?.is_default).toBe(true);
    expect(assignments).toEqual([
      secondOverride,
      { flow_id: 5, is_default: true, group_ids: [] },
    ]);
  });
});

describe('overrideKey', () => {
  it('separates two overrides that share a flow', () => {
    const sameFlowOther: MfaFlowAssignment = {
      flow_id: firstOverride.flow_id,
      is_default: false,
      group_ids: [12],
    };
    expect(overrideKey(firstOverride)).not.toBe(overrideKey(sameFlowOther));
  });
});

describe('selectableFlows', () => {
  const catalog = [{ id: 1 }, { id: 2 }, { id: 3 }];

  it('hides flows that are already assigned', () => {
    const assignments = [defaultFlow, firstOverride];
    expect(selectableFlows(catalog, assignments, undefined)).toEqual([{ id: 3 }]);
  });

  it('keeps the flow the editor is holding', () => {
    const assignments = [defaultFlow, firstOverride];
    expect(selectableFlows(catalog, assignments, firstOverride.flow_id)).toEqual([
      { id: 2 },
      { id: 3 },
    ]);
  });
});
