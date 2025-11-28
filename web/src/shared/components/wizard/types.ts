export interface WizardPageConfig {
  title: string;
  subtitle: string;
  activeStep: number;
  steps: WizardPageStep[];
  relatedDocs?: WizardDocsLink[];
}

export interface WizardDocsLink {
  link: string;
  label: string;
}

export interface WizardPageStep {
  id: number;
  label: string;
  description?: string;
}
