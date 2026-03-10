import { Button } from '../../../defguard-ui/components/Button/Button';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';
import { Controls } from '../../Controls/Controls';
import { WizardCard } from '../../wizard/WizardCard/WizardCard';
import './style.scss';

export interface WizardStepSummaryRecommendation {
  iconSrc: string;
  iconAlt: string;
  kicker: string;
  title: string;
  buttonText: string;
  onButtonClick: () => void;
}

interface WizardStepSummaryProps {
  thankYouText: string;
  noteText: string;
  ports: string[];
  encourageText: string;
  recommendations: WizardStepSummaryRecommendation[];
  submitButtonText: string;
  onSubmit: () => void;
  submitLoading?: boolean;
  className?: string;
}

export const WizardStepSummary = ({
  thankYouText,
  noteText,
  ports,
  encourageText,
  recommendations,
  submitButtonText,
  onSubmit,
  submitLoading = false,
  className,
}: WizardStepSummaryProps) => {
  const cardClassName = className
    ? `wizard-step-summary ${className}`
    : 'wizard-step-summary';

  return (
    <WizardCard className={cardClassName}>
      <p className="thank-you">{thankYouText}</p>
      <Divider spacing={ThemeSpacing.Xl} />
      <p className="note">{noteText}</p>
      <SizedBox height={ThemeSpacing.Lg} />
      <ul>
        {ports.map((port) => (
          <li key={port}>{port}</li>
        ))}
      </ul>
      <Divider spacing={ThemeSpacing.Xl2} />
      <p className="encourage">{encourageText}</p>
      <SizedBox height={ThemeSpacing.Md} />
      <div className="recommendations">
        {recommendations.map((recommendation, index) => (
          <div className="container" key={`${recommendation.title}-${index}`}>
            <img
              src={recommendation.iconSrc}
              alt={recommendation.iconAlt}
              className="icon"
            />
            <div className="recommendation-row">
              <div className="kicker-title">
                <p className="kicker">{recommendation.kicker}</p>
                <p className="title">{recommendation.title}</p>
              </div>
              <Button
                variant="outlined"
                text={recommendation.buttonText}
                iconRight="open-in-new-window"
                onClick={recommendation.onButtonClick}
              />
            </div>
          </div>
        ))}
      </div>

      <Controls>
        <div className="right">
          <Button text={submitButtonText} onClick={onSubmit} loading={submitLoading} />
        </div>
      </Controls>
    </WizardCard>
  );
};
