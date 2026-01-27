export const GatewaySetupStep = {
  GatewayComponent: 'gatewayComponent',
  GatewayAdaptation: 'gatewayAdaptation',
  Confirmation: 'confirmation',
} as const;

export type GatewaySetupStepValue =
  (typeof GatewaySetupStep)[keyof typeof GatewaySetupStep];
