import { useMutation } from '@tanstack/react-query';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../../paraglide/messages';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../../../shared/defguard-ui/components/Toggle/Toggle';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { validateExternalProviderWizard } from '../../consts';
import { useAddExternalOpenIdStore } from '../../useAddExternalOpenIdStore';

export const AddExternalOpenIdDirectoryStep = () => {
  const [back, next] = useAddExternalOpenIdStore(useShallow((s) => [s.back, s.next]));

  const { mutate, isPending } = useMutation({
    mutationFn: validateExternalProviderWizard,
    onSuccess: (result) => {
      if (typeof result === 'boolean') {
        useAddExternalOpenIdStore.setState({
          testResult: result,
        });
        next();
      } else {
        useAddExternalOpenIdStore.setState({
          testResult: result.success,
          testMessage: result.message,
        });
        next();
      }
    },
  });

  return (
    <WizardCard id="add-external-openid-directory-step">
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {
          "You can optionally enable directory synchronization to automatically sync user's status and groups from an external provider."
        }
      </AppText>
      <SizedBox height={ThemeSpacing.Xl} />
      <Toggle active={false} disabled={true} label="Directory synchronization" />
      <SizedBox height={ThemeSpacing.Xl} />
      <Divider />
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            back();
          }}
        />
        <div className="right">
          <Button
            variant="primary"
            text={m.controls_continue()}
            loading={isPending}
            onClick={() => {
              mutate();
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
