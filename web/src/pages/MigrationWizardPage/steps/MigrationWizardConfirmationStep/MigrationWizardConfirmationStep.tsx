import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { externalLink } from '../../../../shared/constants';
import { ActionableSection } from '../../../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { RenderMarkdown } from '../../../../shared/defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import {
  getLocationsQueryOptions,
  getSessionInfoQueryOptions,
} from '../../../../shared/query';
import prepareNetworkImage from './assets/prepare-network.png';

export const MigrationWizardConfirmationStep = () => {
  const [confirm, setConfirm] = useState(false);
  const navigate = useNavigate();

  const { data: sessionInfo } = useQuery(getSessionInfoQueryOptions);
  const { data: locations } = useQuery(getLocationsQueryOptions);

  const { mutate: finish, isPending } = useMutation({
    mutationFn: api.migration.finish,
    onSuccess: async () => {
      Snackbar.success(`Migration finished`);
    },
    onError: (e) => {
      Snackbar.error(`Finishing migration failed`);
      console.error(e);
    },
    meta: {
      invalidate: [['migration'], ['settings'], ['session-info'], ['me']],
    },
  });

  useEffect(() => {
    if (sessionInfo?.wizard_flags) {
      if (sessionInfo.wizard_flags.migration_wizard_completed) {
        navigate({ to: '/vpn-overview', replace: true });
      }
    }
  }, [sessionInfo, navigate]);

  return (
    <WizardCard>
      <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgSuccess}>
        {m.migration_wizard_confirmation_title()}
      </AppText>
      <SizedBox height={ThemeSpacing.Sm} />
      <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgNeutral}>
        {m.migration_wizard_confirmation_subtitle()}
      </AppText>
      <Divider spacing={ThemeSpacing.Xl2} />
      <AppText font={TextStyle.TBodyPrimary500} color={ThemeVariable.FgFaded}>
        {m.migration_wizard_confirmation_locations_info({
          count: locations?.length ?? '',
        })}
      </AppText>
      <SizedBox height={ThemeSpacing.Md} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
        {m.migration_wizard_confirmation_architecture_change_info()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <ul id="upgrade-guide-list">
        <li>{m.migration_wizard_confirmation_rule_1()}</li>
        <li>{m.migration_wizard_confirmation_rule_2()}</li>
      </ul>
      <Divider spacing={ThemeSpacing.Lg} />
      <RenderMarkdown
        containerProps={{
          id: 'confirm-improve-notice',
        }}
        content={m.migration_wizard_confirmation_security_notice_markdown({
          link: externalLink.defguard.docs,
        })}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <ActionableSection
        imageSrc={prepareNetworkImage}
        title={m.migration_wizard_confirmation_prepare_network_title()}
        subtitle={m.migration_wizard_confirmation_prepare_network_subtitle()}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <Checkbox
        active={confirm}
        onClick={() => {
          setConfirm((s) => !s);
        }}
        text={m.migration_wizard_confirmation_checkbox_label()}
      />
      <Controls>
        <div className="right">
          <Button
            variant="primary"
            text={m.controls_finish()}
            disabled={!confirm}
            loading={isPending}
            onClick={() => {
              finish();
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
