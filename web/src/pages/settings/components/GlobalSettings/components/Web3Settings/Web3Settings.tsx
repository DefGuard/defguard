import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { ChangeEvent, useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Textarea } from '../../../../../../shared/defguard-ui/components/Layout/Textarea/Textarea';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../shared/queries';
import { useSettingsPage } from '../../../../hooks/useSettingsPage';

export const Web3Settings = () => {
  const { LL } = useI18nContext();
  const settings = useSettingsPage((state) => state.settings);
  const [signMessage, setSignMessage] = useState(settings?.challenge_template ?? '');

  const {
    settings: { patchSettings },
  } = useApi();

  const queryClient = useQueryClient();

  const toaster = useToaster();

  const { mutate, isLoading } = useMutation(patchSettings, {
    onSuccess: () => {
      toaster.success(LL.settingsPage.messages.challengeSuccess());
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
    },
    onError: (err) => {
      console.error(err);
      toaster.error(LL.messages.error());
    },
  });

  return (
    <section id="web3-settings">
      <header>
        <h2>{LL.settingsPage.web3Settings.header()}</h2>
      </header>
      <Card shaded bordered>
        <div className="controls">
          <h3>{LL.settingsPage.web3Settings.fields.signMessage.label()}:</h3>
          <Button
            text={LL.settingsPage.web3Settings.controls.save()}
            icon={<IconCheckmarkWhite />}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            loading={isLoading}
            disabled={signMessage.length < 4}
            onClick={() => {
              if (signMessage) {
                mutate({ challenge_template: signMessage });
              }
            }}
          />
        </div>
        <div className="text-wrap">
          <div className="scroll-wrapper">
            {!isUndefined(signMessage) && (
              <Textarea
                value={signMessage}
                onChange={(e: ChangeEvent<HTMLTextAreaElement>) =>
                  setSignMessage(e.target.value)
                }
                disabled={isLoading}
              />
            )}
          </div>
        </div>
      </Card>
    </section>
  );
};
