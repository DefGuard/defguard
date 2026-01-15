import { m } from '../../paraglide/messages';
import { Card } from '../../shared/components/Card/Card';
import { CodeCard } from '../../shared/defguard-ui/components/CodeCard/CodeCard';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import './style.scss';
import { useState } from 'react';
import { Controls } from '../../shared/components/Controls/Controls';
import { LoadingStep } from '../../shared/components/LoadingStep/LoadingStep';
import { ActionableSection } from '../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { ActionableSectionVariant } from '../../shared/defguard-ui/components/ActionableSection/types';
import { BadgeVariant } from '../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../shared/defguard-ui/components/Checkbox/Checkbox';
import { CheckboxIndicator } from '../../shared/defguard-ui/components/CheckboxIndicator/CheckboxIndicator';
import { Radio } from '../../shared/defguard-ui/components/Radio/Radio';
import { RadioIndicator } from '../../shared/defguard-ui/components/RadioIndicator/RadioIndicator';
import { SectionSelect } from '../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { FoldableRadioSection } from '../FoldableRadioSection/FoldableRadioSection';
import testIconSrc from './assets/actionable-test1.png';

export const PlaygroundPage = () => {
  return (
    <div id="playground-page">
      <Card>
        <CodeCard title="Code section title" value={m.test_placeholder_extreme()} />
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <LoadingStepsTest />
      <Divider spacing={ThemeSpacing.Sm} />
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
      <Divider spacing={ThemeSpacing.Sm} />
      <Card>
        <CheckboxIndicator active={false} />
        <Divider spacing={ThemeSpacing.Xl} />
        <CheckboxIndicator active={true} />
        <Divider spacing={ThemeSpacing.Xl} />
        <CheckboxIndicator active={false} error />
        <Divider spacing={ThemeSpacing.Xl} />
        <CheckboxIndicator active={false} disabled />
        <Divider spacing={ThemeSpacing.Xl} />
        <CheckboxIndicator active={true} disabled />
        <Divider spacing={ThemeSpacing.Xl} />
        <CheckboxIndicator active={false} error disabled />
        <Divider spacing={ThemeSpacing.Xl} />
        <Checkbox text={m.test_placeholder_long()} />
        <Divider spacing={ThemeSpacing.Xl} />
        <Checkbox text={m.test_placeholder_long()} error={m.test_placeholder()} />
        <Divider spacing={ThemeSpacing.Xl} />
        <Checkbox text={m.test_placeholder_long()} disabled />
        <Divider spacing={ThemeSpacing.Xl} />
        <Checkbox text={m.test_placeholder_long()} active />
        <Divider spacing={ThemeSpacing.Xl} />
        <Checkbox text={m.test_placeholder_long()} active disabled />
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <Card>
        <RadioIndicator />
        <Divider spacing={ThemeSpacing.Xl} />
        <RadioIndicator disabled />
        <Divider spacing={ThemeSpacing.Xl} />
        <RadioIndicator active />
        <Divider spacing={ThemeSpacing.Xl} />
        <Radio text={m.test_placeholder_long()} />
        <Divider spacing={ThemeSpacing.Xl} />
        <Radio text={m.test_placeholder_long()} active />
        <Divider spacing={ThemeSpacing.Xl} />
        <Radio text={m.test_placeholder_long()} disabled />
        <Divider spacing={ThemeSpacing.Xl} />
        <Radio text={m.test_placeholder_long()} error={m.test_placeholder()} />
        <Divider spacing={ThemeSpacing.Xl} />
        <Radio text={m.test_placeholder_long()} active error={m.test_placeholder()} />
        <Divider spacing={ThemeSpacing.Xl} />
        <Radio text={m.test_placeholder_long()} disabled error={m.test_placeholder()} />
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <FoldSectionTest />
      <Divider spacing={ThemeSpacing.Sm} />
      <Card>
        <SectionSelect
          title={m.test_placeholder()}
          content={m.test_placeholder_long()}
          image="location"
          checkbox
        />
        <Divider spacing={ThemeSpacing.Xl} />
        <SectionSelect
          title={m.test_placeholder()}
          content={m.test_placeholder_long()}
          image="location"
          selected
          checkbox
        />
      </Card>
    </div>
  );
};

const FoldSectionTest = () => {
  const [selected, setSelected] = useState(false);
  return (
    <Card>
      <FoldableRadioSection
        active={selected === true}
        title="Create a certificate authority & configure all Defguard components"
        subtitle={`By choosing this option, Defguard will create its own certificate authority and automatically configure all components to use its certificates — no manual setup required.`}
        badge={{
          variant: BadgeVariant.Success,
          text: 'Recommended',
        }}
        onClick={() => {
          setSelected(true);
        }}
      >
        <Button text={m.test_placeholder()} />
      </FoldableRadioSection>
      <SizedBox height={ThemeSpacing.Xl3} />
      <FoldableRadioSection
        active={selected === false}
        title="Use my own certificate authority"
        subtitle={`If you choose this option, you'll need to manually configure all Defguard components to use your own certificate authority by providing the required certificates and keys during deployment. Use this only if you already manage a private CA — though we still recommend the option above for better security and a dedicated CA for Defguard.`}
        onClick={() => {
          setSelected(false);
        }}
      >
        <Button variant="outlined" text={m.test_placeholder()} />
      </FoldableRadioSection>
    </Card>
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
