import type { AclRule } from '../api/types';

// An edit or a delete of an applied rule adds a pending copy with `parent_id` set, so the
// rows are grouped by `parent_id ?? id` and each group shows the name of its applied rule.
export const ruleNamesByResourceId = (
  rules: AclRule[],
  pick: (rule: AclRule) => number[],
): Record<number, string[]> => {
  const lineages = new Map<number, { name: string; resourceIds: Set<number> }>();
  rules.forEach((rule) => {
    const key = rule.parent_id ?? rule.id;
    const lineage = lineages.get(key) ?? { name: rule.name, resourceIds: new Set() };
    if (rule.parent_id === null) {
      lineage.name = rule.name;
    }
    pick(rule).forEach((resourceId) => {
      lineage.resourceIds.add(resourceId);
    });
    lineages.set(key, lineage);
  });

  const map: Record<number, string[]> = {};
  lineages.forEach(({ name, resourceIds }) => {
    resourceIds.forEach((resourceId) => {
      if (!map[resourceId]) {
        map[resourceId] = [];
      }
      map[resourceId].push(name);
    });
  });
  return map;
};
