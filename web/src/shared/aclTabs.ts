import { z } from 'zod';

export const AclListTab = {
  Deployed: 'deployed',
  Pending: 'pending',
} as const;

const defaultAclListTab = AclListTab.Deployed;

export const aclListTabSchema = z
  .enum([AclListTab.Deployed, AclListTab.Pending])
  .catch(defaultAclListTab);

export type AclListTabValue = z.infer<typeof aclListTabSchema>;

export const aclListRouteSearchSchema = z.object({
  tab: aclListTabSchema.default(defaultAclListTab),
});

export const aclFlowRouteSearchSchema = z.object({
  tab: aclListTabSchema.optional(),
});

export const getCanonicalAclListUrlSearch = (tab: AclListTabValue): string => {
  return `?${new URLSearchParams({ tab }).toString()}`;
};
