import './style.scss';

import parse from 'html-react-parser';
import { UseFormReturn } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { OpenIdProvider } from '../../../../../shared/types';

export const OpenIdGeneralSettings = ({
  formControl,
  isLoading,
}: {
  formControl: UseFormReturn<
    OpenIdProvider & {
      create_account: boolean;
    }
  >;
  isLoading: boolean;
}) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;

  return (
    <section id="openid-settings">
      <header>
        <h2>{localLL.general.title()}</h2>
        <Helper>{parse(localLL.general.helper())}</Helper>
      </header>
      <div>
        <div>
          <div className="checkbox-row">
            <FormCheckBox
              label={localLL.general.createAccount.label()}
              controller={{
                control: formControl.control,
                name: 'create_account',
              }}
              labelPlacement="right"
              disabled={isLoading}
            />
            <Helper>{localLL.general.createAccount.helper()}</Helper>
          </div>
        </div>
      </div>
    </section>
  );
};
