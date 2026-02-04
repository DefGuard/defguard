export const GatewaySetupStep = {
  GatewayComponent: 'gatewayComponent',
  GatewayAdoption: 'gatewayAdoption',
  Confirmation: 'confirmation',
} as const;

export type GatewaySetupStepValue =
  (typeof GatewaySetupStep)[keyof typeof GatewaySetupStep];
