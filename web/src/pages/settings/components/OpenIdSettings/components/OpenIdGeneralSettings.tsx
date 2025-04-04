import './style.scss';

import parse from 'html-react-parser';
import { useFormContext, useWatch } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';

export const OpenIdGeneralSettings = ({ isLoading }: { isLoading: boolean }) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const { control, setValue } = useFormContext();
  const create_account = useWatch({
    control,
    name: 'create_account',
  }) as boolean;

  return (
    <div id="general-settings">
      <div className="subsection-header helper-row">
        <h3>{localLL.general.title()}</h3>
        <Helper>{parse(localLL.general.helper())}</Helper>
      </div>
      <div>
        <div>
          <div className="helper-row">
            {/* FIXME: Really buggy when using the controller, investigate why */}
            <LabeledCheckbox
              label={localLL.general.createAccount.label()}
              // controller={{
              //   control,
              //   name: 'create_account',
              // }}
              value={create_account}
              onChange={(e) => {
                setValue('create_account', e);
              }}
              disabled={isLoading}
            />
            <Helper>{localLL.general.createAccount.helper()}</Helper>
          </div>
        </div>
      </div>
    </div>
  );
};
