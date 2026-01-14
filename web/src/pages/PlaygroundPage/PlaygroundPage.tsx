import { m } from '../../paraglide/messages';
import { Card } from '../../shared/components/Card/Card';
import { CodeCard } from '../../shared/defguard-ui/components/CodeCard/CodeCard';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import './style.scss';
import { Controls } from '../../shared/components/Controls/Controls';
import { LoadingStep } from '../../shared/components/LoadingStep/LoadingStep';
import { ActionableSection } from '../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { ActionableSectionVariant } from '../../shared/defguard-ui/components/ActionableSection/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import testIconSrc from './assets/actionable-test1.png';

export const PlaygroundPage = () => {
  return (
    <div id="playground-page">
      <Card>
        <CodeCard title="Code section title" value={m.test_placeholder_extreme()} />
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <LoadingStepsTest />
      <Card>
        <ActionableSection
          title={m.test_placeholder()}
          subtitle={m.test_placeholder_long()}
          imageSrc={testIconSrc}
        />
        <Divider spacing={ThemeSpacing.Xl} />
        <ActionableSection
          variant={ActionableSectionVariant.Secondary}
          title={m.test_placeholder()}
          subtitle={m.test_placeholder_extreme()}
          imageSrc={testIconSrc}
        />
        <Divider spacing={ThemeSpacing.Xl} />
        <ActionableSection
          variant={ActionableSectionVariant.Secondary}
          title={m.test_placeholder()}
          subtitle={m.test_placeholder_extreme()}
          imageSrc={testIconSrc}
        >
          <Button text={m.test_placeholder()} iconRight="open-in-new-window" />
        </ActionableSection>
      </Card>
    </div>
  );
};

const LoadingStepsTest = () => {
  return (
    <>
      <Card>
        <div>
          <LoadingStep loading title={m.test_placeholder_long()} />
          <LoadingStep title={m.test_placeholder_long()} />
          <LoadingStep title={m.test_placeholder_long()} />
          <LoadingStep title={m.test_placeholder_long()} />
          <LoadingStep title={m.test_placeholder_long()} />
          <LoadingStep title={m.test_placeholder_long()} />
        </div>
      </Card>
      <Card>
        <div>
          <LoadingStep success title={m.test_placeholder_long()} />
          <LoadingStep success title={m.test_placeholder_long()} />
          <LoadingStep success title={m.test_placeholder_long()} />
          <LoadingStep loading title={m.test_placeholder_long()} />
        </div>
      </Card>
      <Card>
        <div>
          <LoadingStep success title={m.test_placeholder_long()} />
          <LoadingStep success title={m.test_placeholder_long()} />
          <LoadingStep
            error
            errorMessage={`Error: ${m.test_placeholder()}`}
            title={m.test_placeholder_long()}
          >
            <CodeCard title="Error log" value={m.test_placeholder_extreme()} />
            <SizedBox height={ThemeSpacing.Xl} />
            <Controls>
              <div className="left">
                <Button variant="primary" text="Retry" disabled />
              </div>
            </Controls>
          </LoadingStep>
          <LoadingStep title={m.test_placeholder_long()} />
        </div>
      </Card>
    </>
  );
};
