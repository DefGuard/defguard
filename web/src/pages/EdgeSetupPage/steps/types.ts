export type SetupEvent = {
  step: SetupStepId;
  proxy_version?: string;
  message?: string;
  logs?: string[];
  error: boolean;
};

export type SetupStep = {
  id: SetupStepId;
  title: string;
};

export type SetupStepId =
  | 'CheckingConfiguration'
  | 'CheckingAvailability'
  | 'CheckingVersion'
  | 'ObtainingCsr'
  | 'SigningCertificate'
  | 'ConfiguringTls'
  | 'Done';
// biome-ignore lint/suspicious/noExplicitAny: SSE hook accepts various data types
export interface SSEHookOptions<T = any> {
  onMessage?: (data: T) => void;
  onError?: (error: Event) => void;
  onOpen?: () => void;
  parseJSON?: boolean;
  params?: Record<string, string | number | boolean>;
}
