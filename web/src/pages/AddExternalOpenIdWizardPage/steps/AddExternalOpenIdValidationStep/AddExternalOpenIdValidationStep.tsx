import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAddExternalOpenIdStore } from '../../useAddExternalOpenIdStore';

export const AddExternalOpenIdValidationStep = () => {
  const navigate = useNavigate();
  const providerState = useAddExternalOpenIdStore((s) => s.providerState);
  const message = useAddExternalOpenIdStore((s) => s.testMessage);
  const result = useAddExternalOpenIdStore((s) => s.testResult);
  const back = useAddExternalOpenIdStore((s) => s.back);

  const { mutate: deleteProvider, isPending } = useMutation({
    mutationFn: api.openIdProvider.deleteOpenIdProvider,
    onSuccess: () => {
      back();
    },
  });

  return (
    <WizardCard id="add-external-openid-validation-step">
      {result && (
        <>
          <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgSuccess}>
            {'External ID provider successfully added.'}
          </AppText>
          <SizedBox height={ThemeSpacing.Sm} />
          <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgNeutral}>
            {
              'The connection to the external identity provider has been successfully verified.'
            }
          </AppText>
          <Divider spacing={ThemeSpacing.Xl} />
          <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgNeutral}>
            {`The connection to your external identity provider is now verified. Your users can now log in using this provider for a faster and more convenient authentication experience.`}
          </AppText>
        </>
      )}
      {!result && (
        <>
          <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgCritical}>
            {'External ID provider successfully added.'}
          </AppText>
          <SizedBox height={ThemeSpacing.Sm} />
          <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgNeutral}>
            {
              'The connection to the external identity provider has been successfully verified.'
            }
          </AppText>
          {isPresent(message) && (
            <>
              <SizedBox height={ThemeSpacing.Xl2} />
              <InfoBanner
                variant="warning"
                icon={IconKind.WarningFilled}
                text={`Error: ${message}`}
              />
            </>
          )}
        </>
      )}
      <Controls>
        {!result && (
          <Button
            variant="outlined"
            text={m.controls_back()}
            loading={isPending}
            onClick={() => {
              deleteProvider(providerState.name);
            }}
          />
        )}
        <div className="right">
          <Button
            variant="primary"
            text={m.controls_finish()}
            disabled={!result || isPending}
            onClick={() => {
              navigate({
                to: '/settings/openid',
                replace: true,
              }).then(() => {
                setTimeout(() => {
                  useAddExternalOpenIdStore.getState().reset();
                }, 100);
              });
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
