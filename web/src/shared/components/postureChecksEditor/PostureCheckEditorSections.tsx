import './style.scss';

import { getOperatingSystemVersionOptionLabel } from '../../../pages/AddPostureCheckWizardPage/operatingSystemVersionLabels';
import { addPostureCheckOperatingSystems } from '../../../pages/AddPostureCheckWizardPage/types';
import type { OperatingSystemConditionKey } from '../../../pages/AddPostureCheckWizardPage/useAddPostureCheckWizardStore';
import {
  type PostureCheckDefguardVersionValue,
  PostureCheckOs,
  type PostureCheckOsValue,
  type PostureCheckVersionValues,
} from '../../../pages/PostureChecksPage/types';
import { m } from '../../../paraglide/messages';
import { Checkbox } from '../../defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { Input } from '../../defguard-ui/components/Input/Input';
import { Select } from '../../defguard-ui/components/Select/Select';
import type { SelectOption } from '../../defguard-ui/components/Select/types';
import { Textarea } from '../../defguard-ui/components/Textarea/Textarea';
import { PolicyOsCard } from '../policyPostures/PolicyOsCard/PolicyOsCard';
import { SystemSelector } from '../SystemSelector/SystemSelector';

export type PostureCheckEditorOperatingSystemState = {
  conditions: OperatingSystemConditionKey[];
  securityUpdates: boolean;
  version: number | null;
};

export type PostureCheckEditorLocationOption = {
  id: number;
  label: string;
};

export type PostureCheckEditorValues = {
  allowPrereleaseClient: boolean;
  configuredOperatingSystems: PostureCheckOsValue[];
  description: string | null;
  locations: Set<number>;
  minimumClientVersion: PostureCheckDefguardVersionValue;
  name: string;
  operatingSystemState: Record<
    PostureCheckOsValue,
    PostureCheckEditorOperatingSystemState
  >;
};

type UpdateValues = (
  updater: (current: PostureCheckEditorValues) => PostureCheckEditorValues,
) => void;

type ConditionDefinition = {
  id: OperatingSystemConditionKey;
  label: string;
};

type GeneralSectionProps = {
  values: PostureCheckEditorValues;
  updateValues: UpdateValues;
};

type OperatingSystemsSectionProps = {
  values: PostureCheckEditorValues;
  versionValues: PostureCheckVersionValues;
  updateValues: UpdateValues;
  compact?: boolean;
};

type DefguardSectionProps = {
  values: PostureCheckEditorValues;
  versionValues: PostureCheckVersionValues;
  updateValues: UpdateValues;
};

type LocationsSectionProps = {
  locationOptions: PostureCheckEditorLocationOption[];
  values: PostureCheckEditorValues;
  updateValues: UpdateValues;
};

const conditionDefinitions = (): Record<PostureCheckOsValue, ConditionDefinition[]> => ({
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
});

const getVersionOptions = (
  operatingSystem: PostureCheckOsValue,
  versionValues: PostureCheckVersionValues,
): SelectOption<number>[] =>
  versionValues[operatingSystem].map((value) => ({
    key: value,
    label: getOperatingSystemVersionOptionLabel(operatingSystem, value),
    value,
  }));

export const PostureCheckGeneralSection = ({
  updateValues,
  values,
}: GeneralSectionProps) => {
  return (
    <div className="posture-check-editor-fields">
      <Input
        required
        label={m.form_label_name()}
        value={values.name}
        onChange={(value) => {
          updateValues((current) => ({
            ...current,
            name: String(value ?? ''),
          }));
        }}
      />
      <Textarea
        label={m.posture_checks_wizard_details_description_optional_label()}
        value={values.description}
        onChange={(value) => {
          updateValues((current) => ({
            ...current,
            description: value,
          }));
        }}
      />
    </div>
  );
};

export const PostureCheckOperatingSystemsSection = ({
  compact = false,
  updateValues,
  values,
  versionValues,
}: OperatingSystemsSectionProps) => {
  const osConditions = conditionDefinitions();
  const visibleSystemSelectors = addPostureCheckOperatingSystems.filter(
    (operatingSystem) => !values.configuredOperatingSystems.includes(operatingSystem),
  );

  return (
    <div className="posture-check-operating-systems">
      {values.configuredOperatingSystems.map((operatingSystem, index) => {
        const details = values.operatingSystemState[operatingSystem];
        const versionOptions = getVersionOptions(operatingSystem, versionValues);
        const selectedVersion =
          versionOptions.find((option) => option.value === details.version) ??
          versionOptions[0];
        const conditions = osConditions[operatingSystem];
        const showWindowsSecurityUpdate = operatingSystem === PostureCheckOs.Windows;

        return (
          <div className="system-item" key={operatingSystem}>
            {index > 0 && <Divider />}
            <PolicyOsCard
              hideCard={compact}
              os={operatingSystem}
              onDiscard={() => {
                updateValues((current) => ({
                  ...current,
                  configuredOperatingSystems: current.configuredOperatingSystems.filter(
                    (value) => value !== operatingSystem,
                  ),
                }));
              }}
            >
              <div className="posture-check-os-card">
                <div className="selects">
                  <div className="select-slot">
                    <Select
                      options={versionOptions}
                      value={selectedVersion}
                      onChange={(option) => {
                        updateValues((current) => ({
                          ...current,
                          operatingSystemState: {
                            ...current.operatingSystemState,
                            [operatingSystem]: {
                              ...current.operatingSystemState[operatingSystem],
                              version: option.value,
                            },
                          },
                        }));
                      }}
                    />
                  </div>
                  {showWindowsSecurityUpdate && (
                    <div className="select-slot">
                      <Select
                        options={[
                          {
                            key: 'outdated',
                            label: 'No requirement',
                            value: false,
                          },
                          {
                            key: 'current',
                            label: 'Updated within 1 month',
                            value: true,
                          },
                        ]}
                        value={
                          details.securityUpdates
                            ? {
                                key: 'current',
                                label: 'Updated within 1 month',
                                value: true,
                              }
                            : {
                                key: 'outdated',
                                label: 'No requirement',
                                value: false,
                              }
                        }
                        onChange={(option) => {
                          updateValues((current) => ({
                            ...current,
                            operatingSystemState: {
                              ...current.operatingSystemState,
                              [operatingSystem]: {
                                ...current.operatingSystemState[operatingSystem],
                                securityUpdates: option.value,
                              },
                            },
                          }));
                        }}
                      />
                    </div>
                  )}
                </div>
                {(showWindowsSecurityUpdate || conditions.length > 0) && (
                  <>
                    <Divider />
                    <div className="conditions">
                      <div className="conditions-copy">
                        <p className="title">
                          {m.posture_checks_wizard_operating_systems_security_conditions()}
                        </p>
                        <p className="description">
                          {m.posture_checks_wizard_operating_systems_security_conditions_description()}
                        </p>
                      </div>
                      <div className="condition-list">
                        {conditions.map((condition) => (
                          <Checkbox
                            key={condition.id}
                            active={details.conditions.includes(condition.id)}
                            text={condition.label}
                            onClick={() => {
                              updateValues((current) => {
                                const currentConditions =
                                  current.operatingSystemState[operatingSystem]
                                    .conditions;
                                const nextConditions = currentConditions.includes(
                                  condition.id,
                                )
                                  ? currentConditions.filter(
                                      (value) => value !== condition.id,
                                    )
                                  : [...currentConditions, condition.id];

                                return {
                                  ...current,
                                  operatingSystemState: {
                                    ...current.operatingSystemState,
                                    [operatingSystem]: {
                                      ...current.operatingSystemState[operatingSystem],
                                      conditions: nextConditions,
                                    },
                                  },
                                };
                              });
                            }}
                          />
                        ))}
                      </div>
                    </div>
                  </>
                )}
              </div>
            </PolicyOsCard>
          </div>
        );
      })}
      {visibleSystemSelectors.length > 0 && (
        <div className="selectors">
          {visibleSystemSelectors.map((operatingSystem) => (
            <SystemSelector
              key={operatingSystem}
              os={operatingSystem}
              onClick={() => {
                updateValues((current) => ({
                  ...current,
                  configuredOperatingSystems: [
                    ...current.configuredOperatingSystems,
                    operatingSystem,
                  ],
                }));
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const PostureCheckDefguardSection = ({
  updateValues,
  values,
  versionValues,
}: DefguardSectionProps) => {
  return (
    <div className="posture-check-defguard">
      <p className="note">{m.posture_checks_edit_defguard_note()}</p>
      <Select
        options={versionValues.defguard.map((version) => ({
          key: version,
          label: m.posture_checks_wizard_client_version_option({ version }),
          value: version,
        }))}
        value={{
          key: values.minimumClientVersion,
          label: m.posture_checks_wizard_client_version_option({
            version: values.minimumClientVersion,
          }),
          value: values.minimumClientVersion,
        }}
        onChange={(option) => {
          updateValues((current) => ({
            ...current,
            minimumClientVersion: option.value,
          }));
        }}
      />
      <Checkbox
        active={values.allowPrereleaseClient}
        onClick={() => {
          updateValues((current) => ({
            ...current,
            allowPrereleaseClient: !current.allowPrereleaseClient,
          }));
        }}
      >
        <div className="checkbox-copy">
          <p className="title">
            {m.posture_checks_wizard_client_version_prerelease_title()}
          </p>
          <p className="description">
            {m.posture_checks_wizard_client_version_prerelease_description()}
          </p>
        </div>
      </Checkbox>
    </div>
  );
};

export const PostureCheckLocationsSection = ({
  locationOptions,
  updateValues,
  values,
}: LocationsSectionProps) => {
  return (
    <div className="posture-check-locations">
      {locationOptions.map((location) => (
        <Checkbox
          key={location.id}
          active={values.locations.has(location.id)}
          text={location.label}
          onClick={() => {
            updateValues((current) => {
              const nextLocations = new Set(current.locations);

              if (nextLocations.has(location.id)) {
                nextLocations.delete(location.id);
              } else {
                nextLocations.add(location.id);
              }

              return {
                ...current,
                locations: nextLocations,
              };
            });
          }}
        />
      ))}
    </div>
  );
};
