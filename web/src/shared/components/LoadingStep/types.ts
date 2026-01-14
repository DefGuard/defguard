import type { PropsWithChildren } from 'react';

export type LoadingStepConfig = {
  title: string;
  error?: boolean;
  errorMessage?: string;
  loading?: boolean;
  success?: boolean;
  testId?: string;
};

export type LoadingStepProps = LoadingStepConfig & PropsWithChildren;
