import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import {
  type PostureCheckEditorValues,
  PostureCheckGeneralSection,
} from '../../../shared/components/postureChecksEditor/PostureCheckEditorSections';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { TextStyle, ThemeVariable } from '../../../shared/defguard-ui/types';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

export const AddPostureCheckDetailsStep = () => {
  const back = useAddPostureCheckWizardStore((s) => s.back);
  const description = useAddPostureCheckWizardStore((s) => s.description);
  const name = useAddPostureCheckWizardStore((s) => s.name);
  const next = useAddPostureCheckWizardStore((s) => s.next);
  const setDescription = useAddPostureCheckWizardStore((s) => s.setDescription);
  const setName = useAddPostureCheckWizardStore((s) => s.setName);
  const allowPrereleaseClient = useAddPostureCheckWizardStore(
    (s) => s.allowPrereleaseClient,
  );
  const configuredOperatingSystems = useAddPostureCheckWizardStore(
    (s) => s.configuredOperatingSystems,
  );
  const minimumClientVersion = useAddPostureCheckWizardStore(
    (s) => s.minimumClientVersion,
  );
  const operatingSystemState = useAddPostureCheckWizardStore(
    (s) => s.operatingSystemState,
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
    <WizardCard className="add-posture-check-details-step">
      <div className="details-track">
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.posture_checks_wizard_details_note()}
        </AppText>
        <PostureCheckGeneralSection
          values={values}
          updateValues={(updater) => {
            const nextValues = updater(values);
            setName(nextValues.name);
            setDescription(nextValues.description);
          }}
        />
      </div>
      <Controls>
        <Button text={m.controls_back()} variant="outlined" onClick={back} />
        <div className="right">
          <Button text={m.controls_continue()} onClick={next} />
        </div>
      </Controls>
    </WizardCard>
  );
};
