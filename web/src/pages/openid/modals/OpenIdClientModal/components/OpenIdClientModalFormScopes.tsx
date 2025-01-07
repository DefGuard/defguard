import { useCallback } from 'react';
import { Control, useController } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { OpenIdClientFormFields, OpenIdClientScope } from '../types';

type Props = {
  control: Control<OpenIdClientFormFields>;
  disabled?: boolean;
};

export const OpenIdClientModalFormScopes = ({ control, disabled = false }: Props) => {
  const { LL } = useI18nContext();
  const {
    field: { value, onChange },
  } = useController({
    control,
    name: 'scope',
  });

  const handleChange = useCallback(
    (change: OpenIdClientScope, current: string[]): void => {
      if (current.includes(change)) {
        onChange(current.filter((v) => v !== change.valueOf()));
      } else {
        onChange([...current, change]);
      }
    },
    [onChange],
  );

  return (
    <div className="scopes">
      <LabeledCheckbox
        data-testid="field-scope-openid"
        label={LL.openidOverview.modals.openidClientModal.form.fields.openid.label()}
        disabled={disabled}
        value={value.includes(OpenIdClientScope.OPENID)}
        onChange={() => handleChange(OpenIdClientScope.OPENID, value)}
      />
      <LabeledCheckbox
        data-testid="field-scope-profile"
        label={LL.openidOverview.modals.openidClientModal.form.fields.profile.label()}
        disabled={disabled}
        value={value.includes(OpenIdClientScope.PROFILE)}
        onChange={() => handleChange(OpenIdClientScope.PROFILE, value)}
      />
      <LabeledCheckbox
        data-testid="field-scope-email"
        label={LL.openidOverview.modals.openidClientModal.form.fields.email.label()}
        disabled={disabled}
        value={value.includes(OpenIdClientScope.EMAIL)}
        onChange={() => handleChange(OpenIdClientScope.EMAIL, value)}
      />
      <LabeledCheckbox
        data-testid="field-scope-phone"
        label={LL.openidOverview.modals.openidClientModal.form.fields.phone.label()}
        disabled={disabled}
        value={value.includes(OpenIdClientScope.PHONE)}
        onChange={() => handleChange(OpenIdClientScope.PHONE, value)}
      />
      <LabeledCheckbox
        data-testid="field-scope-groups"
        label={LL.openidOverview.modals.openidClientModal.form.fields.groups.label()}
        disabled={disabled}
        value={value.includes(OpenIdClientScope.GROUPS)}
        onChange={() => handleChange(OpenIdClientScope.GROUPS, value)}
      />
    </div>
  );
};
