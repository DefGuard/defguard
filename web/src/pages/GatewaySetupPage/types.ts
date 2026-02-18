export const GatewaySetupStep = {
  DeployGateway: 'deployGateway',
  GatewayComponent: 'gatewayComponent',
  GatewayAdoption: 'gatewayAdoption',
  Confirmation: 'confirmation',
} as const;

export type GatewaySetupStepValue =
  (typeof GatewaySetupStep)[keyof typeof GatewaySetupStep];
