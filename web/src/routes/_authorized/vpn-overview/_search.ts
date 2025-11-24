import z from 'zod';

export const vpnOverviewSearchSchema = z.object({
  period: z.number().int().default(1),
});
