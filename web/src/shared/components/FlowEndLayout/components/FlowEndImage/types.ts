export const FlowEndImageVariant = {
  AppError: 'app-error',
  Error404: '404',
} as const;

export type FlowEndImageVariantValue =
  (typeof FlowEndImageVariant)[keyof typeof FlowEndImageVariant];
