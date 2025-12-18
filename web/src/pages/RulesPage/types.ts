export const RulesPageTab = {
  Deployed: 'deployed',
  Pending: 'pending',
} as const;

export type RulesPageTabValue = (typeof RulesPageTab)[keyof typeof RulesPageTab];
