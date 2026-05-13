import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { PolicyOsCard } from '../../../shared/components/policyPostures/PolicyOsCard/PolicyOsCard';
import { SystemSelector } from '../../../shared/components/SystemSelector/SystemSelector';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { EvenSplit } from '../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { FieldLabel } from '../../../shared/defguard-ui/components/FieldLabel/FieldLabel';
import { Select } from '../../../shared/defguard-ui/components/Select/Select';
import type { SelectOption } from '../../../shared/defguard-ui/components/Select/types';
import { TextStyle, ThemeVariable } from '../../../shared/defguard-ui/types';
import {
  PostureCheckOs,
  type PostureCheckOsValue,
  postureCheckVersionValues,
} from '../../PostureChecksPage/types';
import { addPostureCheckOperatingSystems } from '../types';
import {
  type OperatingSystemConditionKey,
  useAddPostureCheckWizardStore,
} from '../useAddPostureCheckWizardStore';

type ConditionDefinition = {
  helperText?: string;
  id: OperatingSystemConditionKey;
  label: string;
};

const updateCadenceValues = ['1d', '1w', '1m'] as const;

const getVersionOptionLabel = (operatingSystem: PostureCheckOsValue, value: string) => {
  switch (operatingSystem) {
    case PostureCheckOs.Windows:
      return `${value} or higher`;
    case PostureCheckOs.Linux:
      return `Kernel ${value} or higher`;
    case PostureCheckOs.Ios:
      return `iOS ${value} or higher`;
    case PostureCheckOs.Android:
      return `Android ${value} or higher`;
    case PostureCheckOs.Macos:
      return `${value} or higher`;
  }
};

export const AddPostureCheckOperatingSystemsStep = () => {
  const next = useAddPostureCheckWizardStore((s) => s.next);
  const configuredOperatingSystems = useAddPostureCheckWizardStore(
    (s) => s.configuredOperatingSystems,
  );
  const addConfiguredOperatingSystem = useAddPostureCheckWizardStore(
    (s) => s.addConfiguredOperatingSystem,
  );
  const removeConfiguredOperatingSystem = useAddPostureCheckWizardStore(
    (s) => s.removeConfiguredOperatingSystem,
  );
  const operatingSystemState = useAddPostureCheckWizardStore(
    (s) => s.operatingSystemState,
  );
  const updateOperatingSystemDetails = useAddPostureCheckWizardStore(
    (s) => s.updateOperatingSystemDetails,
  );

  const updateCadenceOptions: SelectOption<string>[] = [
    {
      key: updateCadenceValues[0],
      label: m.posture_checks_wizard_operating_systems_updates_1_day(),
      value: updateCadenceValues[0],
    },
    {
      key: updateCadenceValues[1],
      label: m.posture_checks_wizard_operating_systems_updates_1_week(),
      value: updateCadenceValues[1],
    },
    {
      key: updateCadenceValues[2],
      label: m.posture_checks_wizard_operating_systems_updates_1_month(),
      value: updateCadenceValues[2],
    },
  ];

  const conditionDefinitions: Record<PostureCheckOsValue, ConditionDefinition[]> = {
    [PostureCheckOs.Windows]: [
      {
        id: 'active-directory',
        label: m.posture_checks_wizard_operating_systems_condition_active_directory(),
      },
      {
        id: 'antivirus',
        label: m.posture_checks_wizard_operating_systems_condition_antivirus(),
      },
      {
        id: 'disk-encryption',
        label: m.posture_checks_wizard_operating_systems_condition_disk_encryption(),
      },
    ],
    [PostureCheckOs.Macos]: [
      {
        id: 'disk-encryption',
        label: m.posture_checks_wizard_operating_systems_condition_disk_encryption(),
      },
      {
        id: 'device-integrity',
        label: m.posture_checks_wizard_operating_systems_condition_device_integrity(),
      },
    ],
    [PostureCheckOs.Linux]: [
      {
        id: 'disk-encryption',
        label: m.posture_checks_wizard_operating_systems_condition_disk_encryption(),
      },
    ],
    [PostureCheckOs.Ios]: [],
    [PostureCheckOs.Android]: [
      {
        id: 'device-integrity',
        label: m.posture_checks_wizard_operating_systems_condition_device_integrity(),
      },
    ],
  };

  const getVersionOptions = (
    operatingSystem: PostureCheckOsValue,
  ): SelectOption<string>[] =>
    postureCheckVersionValues[operatingSystem].map((value) => ({
      key: value,
      label: getVersionOptionLabel(operatingSystem, value),
      value,
    }));

  const toggleCondition = (
    operatingSystem: PostureCheckOsValue,
    condition: OperatingSystemConditionKey,
  ) => {
    const nextConditions = operatingSystemState[operatingSystem].conditions.includes(
      condition,
    )
      ? operatingSystemState[operatingSystem].conditions.filter(
          (value) => value !== condition,
        )
      : [...operatingSystemState[operatingSystem].conditions, condition];

    updateOperatingSystemDetails(operatingSystem, { conditions: nextConditions });
  };

  const renderOperatingSystemCard = (operatingSystem: PostureCheckOsValue) => {
    const versionOptions = getVersionOptions(operatingSystem);
    const details = operatingSystemState[operatingSystem];
    const selectedVersion =
      versionOptions.find((option) => option.value === details.version) ??
      versionOptions[0];
    const selectedUpdateCadence =
      updateCadenceOptions.find((option) => option.value === details.updateCadence) ??
      updateCadenceOptions[2];
    const conditions = conditionDefinitions[operatingSystem];
    const showWindowsSecurityUpdate = operatingSystem === PostureCheckOs.Windows;

    return (
      <PolicyOsCard
        os={operatingSystem}
        onDiscard={() => {
          removeConfiguredOperatingSystem(operatingSystem);
        }}
      >
        <div className="system-details">
          <EvenSplit parts={showWindowsSecurityUpdate ? 2 : 1}>
            <Select
              options={versionOptions}
              value={selectedVersion}
              onChange={(option) => {
                updateOperatingSystemDetails(operatingSystem, { version: option.value });
              }}
            />
            {showWindowsSecurityUpdate && (
              <Select
                options={updateCadenceOptions}
                value={selectedUpdateCadence}
                onChange={(option) => {
                  updateOperatingSystemDetails(operatingSystem, {
                    updateCadence: option.value,
                  });
                }}
              />
            )}
          </EvenSplit>
          {showWindowsSecurityUpdate && (
            <Checkbox
              active={details.securityUpdates}
              onClick={() => {
                updateOperatingSystemDetails(operatingSystem, {
                  securityUpdates: !details.securityUpdates,
                });
              }}
            >
              <div className="checkbox-copy">
                <AppText font={TextStyle.TBodySm500}>
                  {m.posture_checks_wizard_operating_systems_windows_security_updates()}
                </AppText>
                <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
                  {m.posture_checks_wizard_operating_systems_windows_security_updates_description()}
                </AppText>
              </div>
            </Checkbox>
          )}
          {showWindowsSecurityUpdate && conditions.length > 0 && <Divider />}
          {conditions.length > 0 && (
            <div className="system-conditions">
              <div className="section-copy">
                <AppText font={TextStyle.TBodySm500}>
                  {m.posture_checks_wizard_operating_systems_security_conditions()}
                </AppText>
                <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
                  {m.posture_checks_wizard_operating_systems_security_conditions_description()}
                </AppText>
              </div>
              <div className="condition-list">
                {conditions.map((condition) => (
                  <Checkbox
                    key={condition.id}
                    active={details.conditions.includes(condition.id)}
                    text={condition.helperText ? undefined : condition.label}
                    onClick={() => {
                      toggleCondition(operatingSystem, condition.id);
                    }}
                  >
                    {condition.helperText && (
                      <FieldLabel text={condition.label} helper={condition.helperText} />
                    )}
                  </Checkbox>
                ))}
              </div>
            </div>
          )}
        </div>
      </PolicyOsCard>
    );
  };

  const visibleSystemSelectors = addPostureCheckOperatingSystems.filter(
    (operatingSystem) => !configuredOperatingSystems.includes(operatingSystem),
  );

  return (
    <WizardCard className="add-posture-check-operating-systems-step">
      <div className="content-track">
        <div className="copy-block">
          <AppText font={TextStyle.TBodySm500}>
            {m.posture_checks_wizard_operating_systems_title()}
          </AppText>
          <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
            {m.posture_checks_wizard_operating_systems_subtitle()}
          </AppText>
        </div>
        {configuredOperatingSystems.map((operatingSystem) =>
          renderOperatingSystemCard(operatingSystem),
        )}
        <div className="systems-list">
          {visibleSystemSelectors.map((operatingSystem) => (
            <SystemSelector
              key={operatingSystem}
              os={operatingSystem}
              onClick={() => {
                addConfiguredOperatingSystem(operatingSystem);
              }}
            />
          ))}
        </div>
      </div>
      <Controls>
        <div className="right">
          <Button text={m.controls_continue()} onClick={next} />
        </div>
      </Controls>
    </WizardCard>
  );
};
