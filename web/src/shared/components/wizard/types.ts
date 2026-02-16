export interface WizardPageConfig {
  title: string;
  subtitle: string;
  activeStep: string | number;
  steps: WizardPageStepsConfig;
  relatedDocs?: WizardDocsLink[];
  welcomePageConfig?: WizardWelcomePageConfig;
  isOnWelcomePage?: boolean;
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

export interface WizardWelcomePageConfig {
  title: string;
  subtitle: string;
  content: React.ReactNode;
  media: React.ReactNode;
  docsLink?: string;
  docsText?: string;
  onClose?: () => void;
}

export type WizardPageStepsConfig = Record<string, WizardPageStep>;
