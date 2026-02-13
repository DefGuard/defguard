import { m } from '../../paraglide/messages';
import { Card } from '../../shared/components/Card/Card';
import { CodeCard } from '../../shared/defguard-ui/components/CodeCard/CodeCard';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import clsx from 'clsx';
import { useMemo, useState } from 'react';
import z from 'zod';
import { Controls } from '../../shared/components/Controls/Controls';
import { DestinationDismissibleBox } from '../../shared/components/DestinationDismissibleBox/DestinationDismissibleBox';
import { DestinationLabel } from '../../shared/components/DestinationLabel/DestinationLabel';
import { IpAssignmentCard } from '../../shared/components/IpAssignmentCard/IpAssignmentCard';
import { IpAssignmentDeviceSection } from '../../shared/components/IpAssignmentDeviceSection/IpAssignmentDeviceSection';
import { LoadingStep } from '../../shared/components/LoadingStep/LoadingStep';
import { SelectionSection } from '../../shared/components/SelectionSection/SelectionSection';
import type {
  SelectionOption,
  SelectionSectionCustomRender,
} from '../../shared/components/SelectionSection/type';
import { ActionableSection } from '../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { ActionableSectionVariant } from '../../shared/defguard-ui/components/ActionableSection/types';
import { Badge } from '../../shared/defguard-ui/components/Badge/Badge';
import {
  type BadgeProps,
  BadgeVariant,
} from '../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { ButtonsGroup } from '../../shared/defguard-ui/components/ButtonsGroup/ButtonsGroup';
import { Checkbox } from '../../shared/defguard-ui/components/Checkbox/Checkbox';
import { CheckboxIndicator } from '../../shared/defguard-ui/components/CheckboxIndicator/CheckboxIndicator';
import { Chip } from '../../shared/defguard-ui/components/Chip/Chip';
import { Helper } from '../../shared/defguard-ui/components/Helper/Helper';
import { Radio } from '../../shared/defguard-ui/components/Radio/Radio';
import { RadioIndicator } from '../../shared/defguard-ui/components/RadioIndicator/RadioIndicator';
import { SectionSelect } from '../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { SuggestedIpInput } from '../../shared/defguard-ui/components/SuggestedIPInput/SuggestedIPInput';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { getLicenseInfoQueryOptions } from '../../shared/query';
import { FoldableRadioSection } from '../FoldableRadioSection/FoldableRadioSection';
import testIconSrc from './assets/actionable-test1.png';

export const PlaygroundPage = () => {
  return (
    <div id="playground-page">
      <TestPlanUpgrade />
      <Divider spacing={ThemeSpacing.Sm} />
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
      <Divider spacing={ThemeSpacing.Sm} />
      <TestSelectionSection />
      <Divider spacing={ThemeSpacing.Sm} />
      <Card>
        <DestinationLabel
          name="Scanner_Brother-Warsaw-office"
          ips="192.168.1.12, 192.168.1.12, 192.168.1.12 192.168.1.129 192.168.1.12,"
          ports="All ports"
          protocols="UPD, ICMP"
        />
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <Card>
        <DestinationDismissibleBox
          onClick={() => {
            Snackbar.default('Clicked');
          }}
          name="Scanner_Brother-Warsaw-office"
          ips="192.168.1.12, 192.168.1.12, 192.168.1.12 192.168.1.129 192.168.1.12,"
          ports="All ports"
          protocols="UPD, ICMP"
        />
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <Card id="chips-test">
        <div>
          <Chip text="SSH" onDismiss={() => {}} size="sm" />
          <Chip text="General server settings" onDismiss={() => {}} size="lg" />
          <Chip text={m.test_placeholder()} size="sm" />
          <Chip text={m.test_placeholder()} size="lg" />
        </div>
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <TestIpAssignmentSection />
      <Divider spacing={ThemeSpacing.Sm} />
      <Card>
        <Helper tooltipProps={{ id: 'custom-helper-id' }}>
          {m.test_placeholder_long()}
        </Helper>
      </Card>
      <Divider spacing={ThemeSpacing.Sm} />
      <TestFileUpload />
    </div>
  );
};

const TestPlanUpgrade = () => {
  const { data: license, isLoading } = useQuery(getLicenseInfoQueryOptions);
  return (
    <Card>
      <h3>{`Licensing modals`}</h3>
      <SizedBox height={ThemeSpacing.Xl4} />
      <ButtonsGroup>
        <Button
          text="Limits reached"
          onClick={() => {
            openModal(ModalName.LimitReached);
          }}
        />
        <Button
          text="Upgrade business"
          onClick={() => {
            openModal(ModalName.UpgradeBusiness);
          }}
        />
        <Button
          text="Upgrade Enterprise"
          onClick={() => {
            openModal(ModalName.UpgradeEnterprise);
          }}
        />
        <Button
          text="License Expired"
          disabled={!isPresent(license)}
          loading={isLoading}
          onClick={() => {
            if (license) {
              openModal(ModalName.LicenseExpired, {
                licenseTier: license.tier,
              });
            }
          }}
        />
      </ButtonsGroup>
    </Card>
  );
};

const TestIpAssignmentSection = () => {
  const [haveErrors, setHaveErrors] = useState(false);
  const [isOpen, setOpen] = useState(false);
  const [input, setInput] = useState<string | null>('14');
  const [input2, setInput2] = useState<string | null>('11');
  const [input3, setInput3] = useState<string | null>('5486:5236');
  return (
    <Card>
      <SizedBox width={600} height={1} />
      <Button
        text={`Toggle Errors (${haveErrors})`}
        variant="outlined"
        onClick={() => {
          setHaveErrors((s) => !s);
        }}
      />
      <SizedBox height={ThemeSpacing.Xl4} />
      <IpAssignmentCard
        title="Paris Office"
        isOpen={isOpen}
        onOpenChange={(val) => setOpen(val ?? false)}
      >
        <IpAssignmentDeviceSection name="MacBook Pro">
          <SuggestedIpInput
            data={{
              modifiable_part: '14',
              network_part: '10.2.12.',
              network_prefix: '24',
            }}
            value={input}
            error={haveErrors ? m.test_placeholder_long() : undefined}
            onChange={(val) => {
              setInput(val);
            }}
          />
          <SuggestedIpInput
            data={{
              modifiable_part: '24',
              network_part: '10.3.12.',
              network_prefix: '24',
            }}
            value={input2}
            onChange={(val) => {
              setInput2(val);
            }}
          />
          <SuggestedIpInput
            data={{
              modifiable_part: '24',
              network_part: '10.3.12.',
              network_prefix: '24',
            }}
            value={input2}
            error={haveErrors ? m.test_placeholder_long() : undefined}
            onChange={(val) => {
              setInput2(val);
            }}
          />
          <SuggestedIpInput
            data={{
              modifiable_part: '',
              network_part: '2001:db8::42::8a2e:',
              network_prefix: '96',
            }}
            value={input3}
            onChange={(val) => {
              setInput3(val);
            }}
          />
        </IpAssignmentDeviceSection>
      </IpAssignmentCard>
    </Card>
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

const selectionItemRender: SelectionSectionCustomRender<number, BadgeProps> = ({
  active,
  onClick,
  option,
}) => {
  return (
    <div
      className={clsx('custom-selection-item', {
        active,
      })}
      onClick={onClick}
    >
      <CheckboxIndicator active={active} /> <p>{option.label}</p>
      <span>{` | `}</span>
      {isPresent(option.meta) && <Badge {...option.meta} />}
    </div>
  );
};

const TestSelectionSection = () => {
  const [selected, setSelected] = useState<Set<number>>(new Set());

  const options = useMemo(
    (): SelectionOption<number, BadgeProps>[] => [
      { id: 1, label: 'Item 1', meta: { text: 'text', variant: 'success' } },
      { id: 2, label: 'Item 2', meta: { text: 'text', variant: 'critical' } },
      { id: 3, label: 'Item 3', meta: { text: 'text', variant: 'success' } },
      { id: 4, label: 'Item 4', meta: { text: 'text', variant: 'critical' } },
      { id: 5, label: 'Item 5', meta: { text: 'text', variant: 'success' } },
      { id: 6, label: 'Item 6', meta: { text: 'text', variant: 'critical' } },
      { id: 7, label: 'Item 7', meta: { text: 'text', variant: 'success' } },
      { id: 8, label: 'Item 8', meta: { text: 'text', variant: 'critical' } },
      { id: 9, label: 'Item 9', meta: { text: 'text', variant: 'success' } },
      { id: 10, label: 'Item 10', meta: { text: 'text', variant: 'critical' } },
      { id: 11, label: 'Item 11', meta: { text: 'text', variant: 'success' } },
      { id: 12, label: 'Item 12', meta: { text: 'text', variant: 'critical' } },
      { id: 13, label: 'Item 13', meta: { text: 'text', variant: 'success' } },
      { id: 14, label: 'Item 14', meta: { text: 'text', variant: 'critical' } },
      { id: 15, label: 'Item 15', meta: { text: 'text', variant: 'success' } },
      { id: 16, label: 'Item 16', meta: { text: 'text', variant: 'critical' } },
      { id: 17, label: 'Item 17', meta: { text: 'text', variant: 'success' } },
      { id: 18, label: 'Item 18', meta: { text: 'text', variant: 'critical' } },
      { id: 19, label: 'Item 19', meta: { text: 'text', variant: 'success' } },
      { id: 20, label: 'Item 20', meta: { text: 'text', variant: 'critical' } },
    ],
    [],
  );

  return (
    <Card>
      <SizedBox width={600} height={1} />
      <h4>Test custom item render for selection section</h4>
      <SizedBox height={ThemeSpacing.Sm} />
      <p>{`Selection: ${Array.from(selected).join(', ')}`}</p>
      <Divider spacing={ThemeSpacing.Xl} />
      <SelectionSection<number, BadgeProps>
        onChange={setSelected}
        options={options}
        renderItem={selectionItemRender}
        selection={selected}
        id="playground-selection-section-test"
      />
    </Card>
  );
};

const TestFileUpload = () => {
  const testFormSchema = z.object({
    test_field: z.file(m.form_error_required()).nullable(),
  });

  type FormFields = z.infer<typeof testFormSchema>;

  const defaultValues: FormFields = {
    test_field: null,
  };

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: testFormSchema,
      onChange: testFormSchema,
    },
  });

  return (
    <Card>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="test_field">
            {(field) => <field.FormUploadField />}
          </form.AppField>
        </form.AppForm>
      </form>
    </Card>
  );
};
