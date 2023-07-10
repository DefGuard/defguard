import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../i18n/i18n-react';
import { Button } from '../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/types';
import { Card } from '../../../shared/components/layout/Card/Card';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import { IconCheckmarkWhite } from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';

export const Web3Settings = () => {
  const { LL } = useI18nContext();
  const [signMessage, setSignMessage] = useState('');
  const settings = useAppStore((state) => state.settings);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const {
    settings: { editSettings },
  } = useApi();
  const queryClient = useQueryClient();
  const toaster = useToaster();
  const { mutate, isLoading } = useMutation([MutationKeys.EDIT_SETTINGS], editSettings, {
    onSuccess: () => {
      toaster.success(LL.settingsPage.messages.challengeSuccess());
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
    },
    onError: (err) => {
      console.error(err);
      toaster.error(LL.messages.error());
    },
  });

  useEffect(() => {
    if (settings) {
      setSignMessage(settings.challenge_template);
    }
  }, [settings, settings?.challenge_template]);

  return (
    <section className="web3-settings">
      <header>
        <h2>{LL.settingsPage.web3Settings.header()}</h2>
        <Helper initialPlacement="right">PLACEHOLDER</Helper>
      </header>
      <Card>
        <header>
          <h3>{LL.settingsPage.web3Settings.fields.signMessage.label()}</h3>
          <div className="controls">
            <Button
              text={
                breakpoint !== 'mobile'
                  ? LL.settingsPage.web3Settings.controls.save()
                  : undefined
              }
              icon={<IconCheckmarkWhite />}
              size={ButtonSize.SMALL}
              styleVariant={ButtonStyleVariant.SAVE}
              loading={isLoading}
              disabled={signMessage.length < 4}
              onClick={() => {
                if (settings && signMessage) {
                  mutate({ ...settings, challenge_template: signMessage });
                }
              }}
            />
          </div>
        </header>
        <textarea
          value={signMessage}
          onChange={(e) => setSignMessage(e.target.value)}
          disabled={isLoading}
        />
      </Card>
    </section>
  );
};
