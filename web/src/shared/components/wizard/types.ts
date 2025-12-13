export interface WizardPageConfig {
  title: string;
  subtitle: string;
  activeStep: string | number;
  steps: WizardPageStepsConfig;
  relatedDocs?: WizardDocsLink[];
}

export interface WizardDocsLink {
  link: string;
  label: string;
}

export interface WizardPageStep {
  id: number | string;
  order: number;
  label: string;
  description?: string;
  hidden?: boolean;
}

export type WizardPageStepsConfig = Record<string, WizardPageStep>;
