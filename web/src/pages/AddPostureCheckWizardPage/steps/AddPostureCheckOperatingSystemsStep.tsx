import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import {
  type PostureCheckEditorValues,
  PostureCheckOperatingSystemsSection,
} from '../../../shared/components/postureChecksEditor/PostureCheckEditorSections';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { TextStyle, ThemeVariable } from '../../../shared/defguard-ui/types';
import type { PostureCheckVersionValues } from '../../PostureChecksPage/types';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

type Props = {
  versionValues: PostureCheckVersionValues;
};

export const AddPostureCheckOperatingSystemsStep = ({ versionValues }: Props) => {
  const next = useAddPostureCheckWizardStore((s) => s.next);
  const allowPrereleaseClient = useAddPostureCheckWizardStore(
    (s) => s.allowPrereleaseClient,
  );
  const configuredOperatingSystems = useAddPostureCheckWizardStore(
    (s) => s.configuredOperatingSystems,
  );
  const description = useAddPostureCheckWizardStore((s) => s.description);
  const minimumClientVersion = useAddPostureCheckWizardStore(
    (s) => s.minimumClientVersion,
  );
  const name = useAddPostureCheckWizardStore((s) => s.name);
  const operatingSystemState = useAddPostureCheckWizardStore(
    (s) => s.operatingSystemState,
  );
  const addConfiguredOperatingSystem = useAddPostureCheckWizardStore(
    (s) => s.addConfiguredOperatingSystem,
  );
  const removeConfiguredOperatingSystem = useAddPostureCheckWizardStore(
    (s) => s.removeConfiguredOperatingSystem,
  );
  const updateOperatingSystemDetails = useAddPostureCheckWizardStore(
    (s) => s.updateOperatingSystemDetails,
  );

  const values: PostureCheckEditorValues = {
    allowPrereleaseClient,
    configuredOperatingSystems,
    description,
    locations: new Set<number>(),
    minimumClientVersion,
    name,
    operatingSystemState,
  };

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
        <PostureCheckOperatingSystemsSection
          compact
          values={values}
          versionValues={versionValues}
          updateValues={(updater) => {
            const nextValues = updater(values);

            configuredOperatingSystems.forEach((operatingSystem) => {
              if (!nextValues.configuredOperatingSystems.includes(operatingSystem)) {
                removeConfiguredOperatingSystem(operatingSystem);
              }
            });

            nextValues.configuredOperatingSystems.forEach((operatingSystem) => {
              if (!configuredOperatingSystems.includes(operatingSystem)) {
                addConfiguredOperatingSystem(operatingSystem);
              }

              updateOperatingSystemDetails(
                operatingSystem,
                nextValues.operatingSystemState[operatingSystem],
              );
            });
          }}
        />
      </div>
      <Controls>
        <div className="right">
          <Button text={m.controls_continue()} onClick={next} />
        </div>
      </Controls>
    </WizardCard>
  );
};
