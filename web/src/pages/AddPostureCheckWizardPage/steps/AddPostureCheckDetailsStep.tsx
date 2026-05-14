import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Input } from '../../../shared/defguard-ui/components/Input/Input';
import { Textarea } from '../../../shared/defguard-ui/components/Textarea/Textarea';
import { TextStyle, ThemeVariable } from '../../../shared/defguard-ui/types';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

export const AddPostureCheckDetailsStep = () => {
  const back = useAddPostureCheckWizardStore((s) => s.back);
  const description = useAddPostureCheckWizardStore((s) => s.description);
  const name = useAddPostureCheckWizardStore((s) => s.name);
  const next = useAddPostureCheckWizardStore((s) => s.next);
  const setDescription = useAddPostureCheckWizardStore((s) => s.setDescription);
  const setName = useAddPostureCheckWizardStore((s) => s.setName);

  return (
    <WizardCard className="add-posture-check-details-step">
      <div className="details-track">
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.posture_checks_wizard_details_note()}
        </AppText>
        <div className="details-fields">
          <Input
            required
            label={m.form_label_name()}
            value={name}
            onChange={(value) => {
              setName(String(value ?? ''));
            }}
          />
          <Textarea
            label={m.posture_checks_wizard_details_description_optional_label()}
            value={description}
            onChange={setDescription}
          />
        </div>
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
